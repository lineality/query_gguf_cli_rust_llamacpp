# query_gguf_cli_rust_llamacpp


# Overall
1. 'install' for cli use
- pick a quick name you can remember (I use 'roto' because it is quick to type)
2. 'setup' with your models, prompts, in modes of combination
3.  call quickly for a quick query: bash: query
- 'make' a mode
- select past modes
- hit enter again to use default mode
- use 'dir' mode to add a directory-tree to your prompt

# Launch with default mode
query_gguf

# Launch with specific mode
query_gguf 1

# Launch with manual mode
query_gguf manual

# query_gguf.rs, a minimal rust cli program, to:

- Allow the user to as quickly as possible with as few steps as possible,
ideally within the first step after lauch, start a query

- read config data from a toml file
(no third party crates!)

- use get cpu-count from os (or that -1) for threads

- use command to start llama.cpp
(see more about values for parameters below)

- possibly launches a new terminal running the following command

- use gpu layers only if the user says they have a gpu setup (likely in config, stetup in wizard

- open config file in editor to modify it by command, maybe: type config


## linux: for small build, use (for me executible is 1.8mb)
```bash
cargo build --profile release-small 
```

## ~Install
Set an executable file as a keyword in the command line interface (CLI) so that entering that keyword calls the executable:

1. Open the bash shell configuration file in a text editor. The configuration file is usually located at ~/.bashrc or ~/.bash_profile. (use whatever edictor: vim, nano, hx (helix), teehee, lapce, etc.)
```bash
hx ~/.bashrc
```
or in some systems it may be called 'b'ash_profile'

2. Add an alias for your executable at the end of the file. Replace your_executable with the name of your executable and /path/to/your_executable with the full path to your executable.
```bash
alias your_keyword='/path/to/your_executable'
```
e.g.
```bash
alias query='/home/COMPUTERNAME/query_gguf/query_gguf'
alias quickchat='/home/COMPUTERNAME/query_gguf/query_gguf'
alias roto='/home/COMPUTERNAME/query_gguf/query_gguf'
```

3. Save and close the text editor. 
- If you used nano, you can do this by pressing: Ctrl x s (control key, x key, s key)
- If you use Helix(hx), Vim(vi), or Teehee: 'i' to type, then esc for normal mode, then :wq to write and quit

4. Reload the bash shell configuration file to apply the changes.
```bash
source ~/.bashrc
```
or bash_profile


