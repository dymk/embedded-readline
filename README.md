Embedded Readline
==================

This is a simple readline implementation for embedded systems, using async APIs
for reading and writing to an `embedded_io_async::{Read, Write}. Supports a fixed sized buffer for storing both user input and command history.

Supports the following keybindings:

* `Ctrl-A` - Move to the beginning of the line.
* `Ctrl-E` - Move to the end of the line.
* `Ctrl-K` - Delete the all characters after the cursor.
* `Ctrl-W` - Delete the word before the cursor.
* `left` / `right` - Move the cursor.
* `up` / `down` - Navigate line history.
* `Backspace` - Delete the character before the cursor.

Usage
-----

```rust
use uart_readline::{readline, Buffers, ReadlineError};
use embedded_io_async::{Read, Write};

async fn main_loop(uart: &mut impl Read + Write) {
    let mut buffers: Buffers<64, 8> = Buffers::default();

    loop {
        uart.write_async(b"> ").await.unwrap();
        let line = readline(uart, &mut buffers).await.unwrap();
        // do something with the line
    }
}
```
