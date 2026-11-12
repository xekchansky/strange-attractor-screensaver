/* Bridging header for the Swift screen saver host.
 *
 * The C ABI itself lives next to the Rust crate so the Android JNI bridge in
 * I11 can include the same file. Keeping one header means the exported symbols
 * and the host always describe the same interface.
 */

#ifndef ATTRACTOR_BRIDGE_H
#define ATTRACTOR_BRIDGE_H

#include "../../rust/crates/attractor-ffi/include/attractor.h"

#endif /* ATTRACTOR_BRIDGE_H */
