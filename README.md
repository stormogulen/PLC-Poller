# Some test projects for webassembly

This folder contains serveral small test programs:
* A simple webserver in Python
* An index.html page that loads the webassembly and shows it's result
* A webassembly component, that polls the PLC and makes an API/HTTP request
* A PLC emulator that generates random numbers, that the webassembly polls

## Setup and run

### The webserver 

The server will host the `index.html` and serve the WebAssembly files.
Start the webserver with `python3 server.py`


### PLC emulator

An emulator that generates random numbers from 0-200.
Navigate to the `plc-emulator` folder and run:
`cargo build` to build and then  `cargo run` to start.


### WebAssembly Component (PLC API Component)

A webassembly that polls the PLC and makes an API/HTTP request
In the main project directory, build the WebAssembly module 
with ` wasm-pack build --target web`. This will generate a `pkg` folder with the dependencies,
i.e.  the WebAssembly module and JavaScript bindings

### Web Interface (index.html)

Open `index.html` in a web browser through the Python server to see the integration in action.
In this case `http://localhost:8000/`

## How It Works

1. The PLC emulator generates random numbers.
2. The WebAssembly component polls the PLC emulator for data.
3. Upon receiving PLC data, the WebAssembly component makes an API request to JSONPlaceholder.
4. The web page displays both the PLC data and the API response.

## Notes

- Ensure all components are running simultaneously for the demo to work correctly.
- The API requests use JSONPlaceholder, which has a limit of 200 todos.

## Requirements

- Rust and Cargo
- wasm-pack
- Python 3

### To build the WebAssembly module:

Ensure you have wasm-pack installed: `cargo install wasm-pack``
Run: `wasm-pack build --target web``

