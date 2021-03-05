// simpleserial.c

#include "simpleserial.h"
#include <stdint.h>
#include "hal.h"


#define MAX_SS_CMDS 16
static int num_commands = 0;

#define MAX_SS_LEN 192

//#define SS_VER_1_0 0
//#define SS_VER_1_1 1
//#define SS_VER_2_0 2


#define CRC 0xA6
uint8_t ss_crc(uint8_t *buf, uint8_t len)
{
	unsigned int k = 0;
	uint8_t crc = 0x00;
	while (len--) {
		crc ^= *buf++;
		for (k = 0; k < 8; k++) {
			crc = crc & 0x80 ? (crc << 1) ^ 0xA6: crc << 1;
		}
	}
	return crc;

}

// [B_STUFF, CMD, SCMD, LEN, B_STUFF, DATA..., CRC, TERM]

//#define SS_VER SS_VER_2_0

#if SS_VER == SS_VER_2_0

void ss_puts(char *x)
{
	do {
		putch(*x);
	} while (*++x);
}

#define FRAME_BYTE 0x00

uint8_t check_version(uint8_t cmd, uint8_t scmd, uint8_t len, uint8_t *data)
{
	uint8_t ver = SS_VER;
	simpleserial_put('r', 1, &ver);
	return SS_ERR_OK;
}

uint8_t stuff_data(uint8_t *buf, uint8_t len)
{
	uint8_t i = 1;
	uint8_t last = 0;
	for (; i < len; i++) {
		if (buf[i] == FRAME_BYTE) {
			buf[last] = i - last;
			last = i;
		}
	}
	return 0x00;
}

uint8_t unstuff_data(uint8_t *buf, uint8_t len)
{
	uint8_t next = buf[0];
	buf[0] = 0x00;
	//len -= 1;
	uint8_t tmp = next;
	while ((next < len) && tmp != 0) {
		tmp = buf[next];
		buf[next] = FRAME_BYTE;
		next += tmp;
	}
	return next;
}

// Set up the SimpleSerial module by preparing internal commands
// This just adds the "v" command for now...
void simpleserial_init()
{
	//simpleserial_addcmd('v', 0, check_version);
}

typedef struct ss_cmd
{
	char c;
	unsigned int len;
	uint8_t (*fp)(uint8_t, uint8_t, uint8_t, uint8_t *);
} ss_cmd;
static ss_cmd commands[MAX_SS_CMDS];

int simpleserial_addcmd(char c, unsigned int len, uint8_t (*fp)(uint8_t, uint8_t, uint8_t, uint8_t*))
{
	if(num_commands >= MAX_SS_CMDS) {
		putch('a');
		return 1;
	}

	if(len >= MAX_SS_LEN) {
		putch('b');
		return 1;
	}

	commands[num_commands].c   = c;
	commands[num_commands].len = len;
	commands[num_commands].fp  = fp;
	num_commands++;

	return 0;
}

void simpleserial_get(void)
{
	uint8_t data_buf[MAX_SS_LEN];
	uint8_t err = 0;

	for (int i = 0; i < 4; i++) {
		data_buf[i] = getch(); //PTR, cmd, scmd, len
		if (data_buf[i] == FRAME_BYTE) {
			err = SS_ERR_FRAME_BYTE;
			goto ERROR;
		}
	}
	uint8_t next_frame = unstuff_data(data_buf, 4);

	// check for valid command
	uint8_t c = 0; 
	for(c = 0; c < num_commands; c++)
	{
		if(commands[c].c == data_buf[1])
			break;
	}

	if (c == num_commands) {
		err = SS_ERR_CMD;
		goto ERROR;
	}

	//check that next frame not beyond end of message
	// account for cmd, scmd, len, data, crc, end of frame
	if ((data_buf[3] + 5) < next_frame) {
		err = SS_ERR_LEN;
		goto ERROR;
	}

	// read in data
	// eq to len + crc + frame end
	int i = 4;
	for (; i < data_buf[3] + 5; i++) {
		data_buf[i] = getch();
		if (data_buf[i] == FRAME_BYTE) {
			err = SS_ERR_FRAME_BYTE;
			goto ERROR;
		}
	}

	//check that final byte is the FRAME_BYTE
	data_buf[i] = getch();
	if (data_buf[i] != FRAME_BYTE) {
		err = SS_ERR_LEN;
		goto ERROR;
	}

	//fully unstuff data now
	unstuff_data(data_buf + next_frame, i - next_frame + 1);

	//calc crc excluding original frame offset and frame end and crc
	uint8_t crc = ss_crc(data_buf+1, i-2);
	if (crc != data_buf[i-1]) {
		err = SS_ERR_CRC;
		goto ERROR;
	}

	err = commands[c].fp(data_buf[1], data_buf[2], data_buf[3], data_buf+4);

ERROR:
	simpleserial_put('e', 0x01, &err);
	return;
}

void simpleserial_put(char c, uint8_t size, uint8_t* output)
{
	uint8_t data_buf[MAX_SS_LEN];
	data_buf[0] = 0x00;
	data_buf[1] = c;
	data_buf[2] = size;
	int i = 0;
	for (; i < size; i++) {
		data_buf[i + 3] = output[i];
	}
	data_buf[i + 3] = ss_crc(data_buf+1, size+2);
	data_buf[i + 4] = 0x00;
	stuff_data(data_buf, i + 5);
	for (int i = 0; i < size + 5; i++) {
		putch(data_buf[i]);
	}
}


#else

typedef struct ss_cmd
{
	char c;
	unsigned int len;
	uint8_t (*fp)(uint8_t*);
} ss_cmd;
static ss_cmd commands[MAX_SS_CMDS];
// Callback function for "v" command.
// This can exist in v1.0 as long as we don't actually send back an ack ("z")
uint8_t check_version(uint8_t *v)
{
	return SS_VER;
}

static char hex_lookup[16] =
{
	'0', '1', '2', '3', '4', '5', '6', '7',
	'8', '9', 'A', 'B', 'C', 'D', 'E', 'F'
};

int hex_decode(int len, char* ascii_buf, uint8_t* data_buf)
{
	for(int i = 0; i < len; i++)
	{
		char n_hi = ascii_buf[2*i];
		char n_lo = ascii_buf[2*i+1];

		if(n_lo >= '0' && n_lo <= '9')
			data_buf[i] = n_lo - '0';
		else if(n_lo >= 'A' && n_lo <= 'F')
			data_buf[i] = n_lo - 'A' + 10;
		else if(n_lo >= 'a' && n_lo <= 'f')
			data_buf[i] = n_lo - 'a' + 10;
		else
			return 1;

		if(n_hi >= '0' && n_hi <= '9')
			data_buf[i] |= (n_hi - '0') << 4;
		else if(n_hi >= 'A' && n_hi <= 'F')
			data_buf[i] |= (n_hi - 'A' + 10) << 4;
		else if(n_hi >= 'a' && n_hi <= 'f')
			data_buf[i] |= (n_hi - 'a' + 10) << 4;
		else
			return 1;
	}

	return 0;
}


// Set up the SimpleSerial module by preparing internal commands
// This just adds the "v" command for now...
void simpleserial_init()
{
	simpleserial_addcmd('v', 0, check_version);
}

int simpleserial_addcmd(char c, unsigned int len, uint8_t (*fp)(uint8_t*))
{
	if(num_commands >= MAX_SS_CMDS)
		return 1;

	if(len >= MAX_SS_LEN)
		return 1;

	commands[num_commands].c   = c;
	commands[num_commands].len = len;
	commands[num_commands].fp  = fp;
	num_commands++;

	return 0;
}

void simpleserial_get(void)
{
	char ascii_buf[2*MAX_SS_LEN];
	uint8_t data_buf[MAX_SS_LEN];
	char c;

	// Find which command we're receiving
	c = getch();

	int cmd;
	for(cmd = 0; cmd < num_commands; cmd++)
	{
		if(commands[cmd].c == c)
			break;
	}

	// If we didn't find a match, give up right away
	if(cmd == num_commands)
		return;

	// Receive characters until we fill the ASCII buffer
	for(int i = 0; i < 2*commands[cmd].len; i++)
	{
		c = getch();

		// Check for early \n
		if(c == '\n' || c == '\r')
			return;

		ascii_buf[i] = c;
	}

	// Assert that last character is \n or \r
	c = getch();
	if(c != '\n' && c != '\r')
		return;

	// ASCII buffer is full: convert to bytes 
	// Check for illegal characters here
	if(hex_decode(commands[cmd].len, ascii_buf, data_buf))
		return;

	// Callback
	uint8_t ret[1];
	ret[0] = commands[cmd].fp(data_buf);
	
	// Acknowledge (if version is 1.1)
#if SS_VER == SS_VER_1_1
	simpleserial_put('z', 1, ret);
#endif
}

void simpleserial_put(char c, uint8_t size, uint8_t* output)
{
	// Write first character
	putch(c);

	// Write each byte as two nibbles
	for(int i = 0; i < size; i++)
	{
		putch(hex_lookup[output[i] >> 4 ]);
		putch(hex_lookup[output[i] & 0xF]);
	}

	// Write trailing '\n'
	putch('\n');
}

#endif