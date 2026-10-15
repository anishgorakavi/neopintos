#include "devices/vga.h"
#include <round.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include "devices/speaker.h"
#include "threads/io.h"
#include "threads/interrupt.h"
#include "threads/vaddr.h"

/* VGA text screen support.  See [FREEVGA] for more information. */

/* Number of columns and rows on the text display. */
#define COL_CNT 80
#define ROW_CNT 25

/* Current cursor position.  (0,0) is in the upper left corner of
   the display. */
static size_t cursorX, cursorY;

/* Attribute value for gray text on a black background. */
#define GRAY_ON_BLACK 0x07

/* Framebuffer.  See [FREEVGA] under "VGA Text Mode Operation".
   The character at (x,y) is fb[y][x][0].
   The attribute at (x,y) is fb[y][x][1]. */
static uint8_t (*frame_buffer)[COL_CNT][2];

static void clear_row (size_t y);
static void cls (void);
static void newline (void);
static void move_cursor (void);
static void find_cursor (size_t *x, size_t *y);

/* Initializes the VGA text display. */
static void init (void)
{
  /* Already initialized? */
  static bool inited;
  if (!inited)
    {
     // VGA text buffer. IBM decided 0xB8000 was the address.
     // https://wiki.osdev.org/VGA_Text_Mode
      frame_buffer = ptov (0xb8000); 
      find_cursor (&cursorX, &cursorY);
      inited = true;
    }
}

/* Writes C to the VGA text display, interpreting control
   characters in the conventional ways.  */
void vga_putc (int c)
{
  /* Disable interrupts to lock out interrupt handlers
     that might write to the console. */
  enum intr_level old_level = intr_disable ();

  init ();

  switch (c)
    {
      case '\n':
        newline ();
        break;

      case '\f':
        cls ();
        break;

      case '\b':
        if (cursorX > 0)
          cursorX--;
        break;

      case '\r':
        cursorX = 0;
        break;

      case '\t':
        cursorX = ROUND_UP (cursorX + 1, 8);
        if (cursorX >= COL_CNT)
          newline ();
        break;

      case '\a':
        intr_set_level (old_level);
        speaker_beep ();
        intr_disable ();
        break;

      default:
        frame_buffer[cursorY][cursorX][0] = c;
        frame_buffer[cursorY][cursorX][1] = GRAY_ON_BLACK;
        if (++cursorX >= COL_CNT)
          newline ();
        break;
    }

  /* Update cursor position. */
  move_cursor ();

  intr_set_level (old_level);
}

/* Clears the screen and moves the cursor to the upper left. */
static void cls (void)
{
  size_t y;

  for (y = 0; y < ROW_CNT; y++)
    clear_row (y);

  cursorX = cursorY = 0;
  move_cursor ();
}

/* Clears row Y to spaces. */
static void clear_row (size_t y)
{
  size_t x;

  for (x = 0; x < COL_CNT; x++)
    {
      frame_buffer[y][x][0] = ' ';
      frame_buffer[y][x][1] = GRAY_ON_BLACK;
    }
}

/* Advances the cursor to the first column in the next line on
   the screen.  If the cursor is already on the last line on the
   screen, scrolls the screen upward one line. */
static void newline (void)
{
  cursorX = 0;
  cursorY++;
  if (cursorY >= ROW_CNT)
    {
      cursorY = ROW_CNT - 1;
      memmove (&frame_buffer[0], &frame_buffer[1], sizeof frame_buffer[0] * (ROW_CNT - 1));
      clear_row (ROW_CNT - 1);
    }
}

/* Moves the hardware cursor to (cx,cy). */
static void move_cursor (void)
{
  /* See [FREEVGA] under "Manipulating the Text-mode Cursor". */
  uint16_t cp = cursorX + COL_CNT * cursorY;
  outw (0x3d4, 0x0e | (cp & 0xff00));
  outw (0x3d4, 0x0f | (cp << 8));
}

/* Reads the current hardware cursor position into (*X,*Y). */
static void find_cursor (size_t *x, size_t *y)
{
  /* See [FREEVGA] under "Manipulating the Text-mode Cursor". */
  uint16_t cp;

  outb (0x3d4, 0x0e);
  cp = inb (0x3d5) << 8;

  outb (0x3d4, 0x0f);
  cp |= inb (0x3d5);

  *x = cp % COL_CNT;
  *y = cp / COL_CNT;
}
