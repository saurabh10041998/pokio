## Pokio

This is me poking Tokio Rust library because of its obsession. 

Plan is to practice some examples first and then do
deep code dive to understand tokio-system.


## Example added
You can see the per example source code under `src/<example-folder-name>/bin/main.rs`

For now to compile and run particular example Run

```bash

cargo run --bin <example-name> [--] [<example-args>]
```

You can Refer the example list and  example command line argument (if any) from below table


Examples added
-----

| Example name | Description | Example Args |
|--------------|:------------:|:------------:|
| chat |  Chat server room demo | No arguments
| connect | tcp and udp connect demo | for udp ```-udp <sock-addr>``` and <br/> for tcp ```<sock-addr>```|
| custom-executor | Custom executor example | No arguments | 
