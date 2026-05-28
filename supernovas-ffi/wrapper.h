#include <novas.h>

#ifdef SUPERNOVAS_FFI_WITH_CALCEPH
/* Pulls in <calceph.h> transitively and declares SuperNOVAS's calceph
 * integration entry points (novas_use_calceph, etc.). Enabled via
 * `-DSUPERNOVAS_FFI_WITH_CALCEPH` from build.rs when the `calceph`
 * cargo feature is on. */
#include <novas-calceph.h>
#endif
