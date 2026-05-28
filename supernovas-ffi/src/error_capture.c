/**
 * Thread-local error message capture for the Rust supernovas wrapper.
 *
 * Installed via novas_set_error_handler() so that the human-readable
 * descriptions produced by novas_error() / novas_set_errno() are captured
 * in-process (rather than written to stderr) and retrievable by Rust via
 * novas_take_c_error().
 */

#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#define NOVAS_CAPTURE_BUF_LEN 512

#if defined(_MSC_VER)
#  define THREAD_LOCAL __declspec(thread)
#else
#  define THREAD_LOCAL __thread
#endif

static THREAD_LOCAL char  capture_buf[NOVAS_CAPTURE_BUF_LEN];
static THREAD_LOCAL int   capture_pos;
static THREAD_LOCAL int   has_capture;

/**
 * SuperNOVAS error handler that appends the formatted message into a
 * thread-local ring-buffer instead of writing to stderr.
 *
 * Installed by calling novas_set_error_handler(novas_capture_handler).
 */
void novas_capture_handler(const char *fmt, va_list args) {
    int remaining = NOVAS_CAPTURE_BUF_LEN - capture_pos;
    if (remaining <= 1) return;  /* buffer full */
    int n = vsnprintf(capture_buf + capture_pos, (size_t)remaining, fmt, args);
    if (n > 0) {
        capture_pos += (n < remaining - 1) ? n : remaining - 1;
        capture_buf[capture_pos] = '\0';
    }
    has_capture = 1;
}

/**
 * Take and clear the captured error string for this thread.
 *
 * Returns a pointer to the internal buffer (valid until the next call to
 * this function or novas_capture_handler), or NULL if no message was captured.
 */
const char *novas_take_c_error(void) {
    if (!has_capture) return 0;
    has_capture = 0;
    capture_pos = 0;
    return capture_buf;
}
