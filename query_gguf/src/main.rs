// roto query_gguf a cli wrapper for llama.cpp chat in rust
// cargo build --profile release-small 
/*

Todo:
1. dir mode


# Overall steps for use:
1. 'install' for cli use (see readme: install llama.cpp and a model, use or build query_gguf, set bash path, put in dir)(pick whatever call names you want)
2. 'setup' with your models, prompts, in modes of combination
3.  call quickly for a quick query: bash: query


# Launch with default mode
query_gguf

# Launch with specific mode
query_gguf 1

# Launch with manual mode
query_gguf manual

# query_gguf.rs, a minimal rust cli program, to:

- ideally operate on linux, macOS, or other prominant non-posix OS

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



Sample toml
```toml
llama_cli_path = "/home/./llama.cpp/build/bin/llama-cli"

logging_enabled = true
log_directory_path = "query_gguf/chatlogs"

gguf_model_directory_1 = "/home/./old_jan/models"

prompt_directory = "prompts"


# Mode 1 - llama3.2 - small quantized version
mode_1 = "/home/./old_jan/models/llama3.2-1b-instruct/Llama-3.2-1B-Instruct-Q6_K_L.gguf|prompts/shortcode.txt|temp=0.8|top_k=40|top_p=0.9|ctx_size=2000|threads=11|gpu_layers=0|interactive_first=true|llama3.2|small quantized version"

# Mode 2 - llama3.2v2 - try2lllllama
mode_2 = "/home/./old_jan/models/llama3.2-1b-instruct/Llama-3.2-1B-Instruct-Q6_K_L.gguf|prompts/shortcode.txt|temp=0.9|top_k=50|top_p=1|ctx_size=5000|threads=11|gpu_layers=2|interactive_first=true|llama3.2v2|try2lllllama"

# Mode 3 - meta3.2 - v3
mode_3 = "/home/./old_jan/models/llama3.2-1b-instruct/Llama-3.2-1B-Instruct-Q6_K_L.gguf|prompts/shortcode.txt|temp=0.8|top_k=40|top_p=0.9|ctx_size=2000|threads=11|gpu_layers=0|interactive_first=true|meta3.2|v3"
```

# cargo.toml

```toml
[package]
name = "query_gguf"
version = "0.1.0"
edition = "2021"

[dependencies]

[profile.release-small]
inherits = "release"
lto = true
codegen-units = 1
strip = "symbols"
panic = "abort"
incremental = false
opt-level = 's'
debug = false
```
*/

use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::{PathBuf, Path};

/// Gets the user's home directory path across different operating systems
/// 
/// This function attempts to find the user's home directory by checking environment
/// variables appropriate for different operating systems:
/// - Linux/MacOS: Uses $HOME
/// - Windows: Uses %USERPROFILE%
/// 
/// # Returns
/// - Ok(String): The absolute path to user's home directory
/// - Err(String): Error message if home directory cannot be determined
/// 
/// # Examples
/// ```
/// match get_home_dir() {
///     Ok(home) => println!("Home directory: {}", home),
///     Err(e) => eprintln!("Could not find home directory: {}", e)
/// }
/// ```
/// 
/// # Error Cases
/// - Environment variables not set
/// - Environment variables contain invalid Unicode
/// 
fn get_home_dir() -> Result<String, String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE")) // Fallback for Windows
        .map_err(|_| "Could not determine home directory".to_string())
}

/// Gets the absolute path to the application's base directory
/// 
/// Creates a 'query_gguf' directory in the user's home directory if it doesn't exist.
/// This directory serves as the base location for all application files including:
/// - Configuration file
/// - Prompt files
/// - Chat logs
/// 
/// # Returns
/// - Ok(PathBuf): Absolute path to the query_gguf directory
/// - Err(String): Error message if directory cannot be created or accessed
/// 
/// # Examples
/// ```
/// match get_app_base_dir() {
///     Ok(path) => println!("App directory: {}", path.display()),
///     Err(e) => eprintln!("Could not access app directory: {}", e)
/// }
/// ```
/// 
/// # Error Cases
/// - Home directory cannot be determined
/// - Insufficient permissions to create directory
/// - Path contains invalid characters
/// 
fn get_app_base_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE")) // Fallback for Windows
        .map_err(|_| "Could not determine home directory".to_string())?;
    
    let base_dir = PathBuf::from(home).join("query_gguf");
    
    // Create the directory if it doesn't exist
    fs::create_dir_all(&base_dir)
        .map_err(|e| format!("Failed to create application directory: {}", e))?;
    
    Ok(base_dir)
}

/// Gets the absolute path to the configuration file
/// 
/// Returns the path to query_gguf_config.toml in the application's base directory:
/// - Linux/MacOS: ~/query_gguf/query_gguf_config.toml
/// - Windows: \Users\username\query_gguf\query_gguf_config.toml
/// 
/// Note: This function does not create the file, it only returns the path where
/// the config file should be located. The file's existence should be checked
/// separately using query_gguf_config_exists().
/// 
/// # Returns
/// - Ok(PathBuf): Absolute path to the configuration file
/// - Err(String): Error message if base directory cannot be accessed
/// 
/// # Examples
/// ```
/// match get_config_path() {
///     Ok(path) => println!("Config file path: {}", path.display()),
///     Err(e) => eprintln!("Could not determine config path: {}", e)
/// }
/// ```
/// 
/// # Error Cases
/// - Base directory cannot be accessed or created
/// - Home directory cannot be determined
/// 
fn get_config_path() -> Result<PathBuf, String> {
    Ok(get_app_base_dir()?.join("query_gguf_config.toml"))
}

/// Gets the absolute path to the prompts directory and ensures it exists
/// 
/// Creates a 'prompts' directory in the application's base directory if it doesn't exist:
/// - Linux/MacOS: ~/query_gguf/prompts/
/// - Windows: \Users\username\query_gguf\prompts\
/// 
/// This directory is used to store all prompt template files that can be
/// used when launching chat sessions. The function ensures the directory
/// exists by creating it if necessary.
/// 
/// # Returns
/// - Ok(PathBuf): Absolute path to the prompts directory
/// - Err(String): Error message if directory cannot be created or accessed
/// 
/// # Examples
/// ```
/// match get_prompts_dir() {
///     Ok(path) => println!("Prompts directory: {}", path.display()),
///     Err(e) => eprintln!("Could not access prompts directory: {}", e)
/// }
/// ```
/// 
/// # Error Cases
/// - Base directory cannot be accessed
/// - Insufficient permissions to create directory
/// - Path contains invalid characters
/// 
fn get_prompts_dir() -> Result<PathBuf, String> {
    let prompts_dir = get_app_base_dir()?.join("prompts");
    
    // Create the prompts directory if it doesn't exist
    fs::create_dir_all(&prompts_dir)
        .map_err(|e| format!("Failed to create prompts directory: {}", e))?;
    
    Ok(prompts_dir)
}

/// Checks if a QueryGGUF configuration file exists at the standard location
/// 
/// Verifies existence of config file at:
/// - Linux/MacOS: ~/query_gguf/query_gguf_config.toml
/// - Windows: \Users\username\query_gguf\query_gguf_config.toml
/// 
/// # Returns
/// - bool: true if config file exists, false otherwise
/// 
fn query_gguf_config_exists() -> bool {
    match get_config_path() {
        Ok(config_path) => config_path.exists(),
        Err(_) => false
    }
}

/// Represents the result of the setup wizard process
#[derive(Debug)]
struct SetupWizardResult {
    gguf_model_directories: Vec<String>,
    prompt_file_directories: Vec<String>,
    log_directory_path: String,
    logging_enabled: bool,
    llama_cpp_directory: String,
}

/// Prompts for llama.cpp executable path during setup
fn setup_llama_cpp_directory() -> Result<String, String> {
    println!("\nLLaMA.cpp Setup:");
    println!("Enter the path to llama-cli executable or its directory");
    println!("(e.g., /path/to/llama.cpp/build/bin/llama-cli");
    println!(" or    /path/to/llama.cpp/build/bin)");
    
    print!("Path to llama.cpp's llama-cli: ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read input: {}", e))?;
    
    let path = input.trim();

    // Normalize the path
    let normalized_path = normalize_path(path)?;
    let normalized_path_buf = PathBuf::from(&normalized_path);

    // Check if the path points directly to llama-cli
    if normalized_path_buf.is_file() {
        if normalized_path_buf.file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.contains("llama-cli"))
            .unwrap_or(false) 
        {
            return Ok(normalized_path);
        }
    }

    // If it's a directory, look for llama-cli inside it
    if normalized_path_buf.is_dir() {
        let cli_path = normalized_path_buf.join("llama-cli");
        if cli_path.exists() && cli_path.is_file() {
            return Ok(cli_path.to_string_lossy().to_string());
        }
    }

    // If we get here, we couldn't find llama-cli
    Err(format!("Could not find llama-cli executable at or in: {}", path))
}

/// Handles the creation and validation of the initial configuration file
/// Returns Result containing either SetupWizardResult or an error message
fn run_query_gguf_setup_wizard() -> Result<SetupWizardResult, String> {
    println!("\n=== Query-GGUF Setup Wizard ===");
    println!("Please answer the following questions to configure Query-gguf.\n");

    let mut wizard_result = SetupWizardResult {
        gguf_model_directories: Vec::new(),
        prompt_file_directories: Vec::new(),
        log_directory_path: String::new(),
        logging_enabled: true,
        llama_cpp_directory: String::new(),
    };

    // Get llama.cpp directory first
    wizard_result.llama_cpp_directory = setup_llama_cpp_directory()?;
    
    // Get model directories
    loop {
        match prompt_for_directory("Enter path to GGUF models directory (or 'done' to finish)") {
            Ok(path) => {
                if path.to_lowercase() == "done" {
                    if wizard_result.gguf_model_directories.is_empty() {
                        println!("Error: At least one model directory is required.");
                        continue;
                    }
                    break;
                }
                wizard_result.gguf_model_directories.push(path);
            }
            Err(e) => {
                println!("Error: {}. Please try again.", e);
            }
        }
    }

    // Get prompt directories
    loop {
        match prompt_for_directory("Enter path to prompt files directory, the default is /query_gguf/prompts (or 'done' to finish)") {
            Ok(path) => {
                if path.to_lowercase() == "done" {
                    break;
                }
                // Replace the old prompt directory loop with this single prompt directory setup
                wizard_result.prompt_file_directories = vec![
                    setup_prompt_directory()?
                ];
            }
            Err(e) => {
                println!("Error: {}. Please try again.", e);
            }
        }
    }
    // Configure logging
    match prompt_yes_no("Enable logging?") {
        Ok(enable_logging) => {
            wizard_result.logging_enabled = enable_logging;
            if enable_logging {
                wizard_result.log_directory_path = setup_log_directory()?;
            }
        }
        Err(e) => return Err(format!("Failed to configure logging: {}", e)),
    }

    // Add default mode setting during initial setup
    let mut config_content = String::new();
    config_content.push_str("default_mode = 1\n\n"); // Set first mode as default
    
    Ok(wizard_result)
}

/// Normalizes a file path to handle both forms (with or without leading slash)
/// Also handles '~' home directory if present
fn normalize_path(path: &str) -> Result<String, String> {
    let path = path.trim();
    
    // Handle home directory expansion if path starts with ~
    let expanded_path = if path.starts_with('~') {
        match std::env::var("HOME") {
            Ok(home) => format!("{}{}", home, &path[1..]),
            Err(_) => return Err("Could not expand home directory (~)".to_string()),
        }
    } else {
        path.to_string()
    };

    // Convert to absolute path if relative
    let path_buf = if expanded_path.starts_with('/') {
        PathBuf::from(expanded_path)
    } else {
        match std::env::current_dir() {
            Ok(cur_dir) => cur_dir.join(expanded_path),
            Err(e) => return Err(format!("Failed to get current directory: {}", e)),
        }
    };

    // Normalize and convert back to string
    match path_buf.canonicalize() {
        Ok(canonical) => match canonical.to_str() {
            Some(s) => Ok(s.to_string()),
            None => Err("Path contains invalid Unicode".to_string()),
        },
        Err(e) => Err(format!("Failed to canonicalize path: {}", e)),
    }
}

/// Modified prompt_for_directory to use path normalization
fn prompt_for_directory(prompt: &str) -> Result<String, String> {
    print!("{}: ", prompt);
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    
    let input = input.trim();
    
    if input.to_lowercase() == "done" {
        return Ok(input.to_string());
    }

    // Normalize the path
    let normalized_path = normalize_path(input)?;
    
    // Verify the normalized path exists and is a directory
    let path_buf = PathBuf::from(&normalized_path);
    if !path_buf.exists() {
        return Err(format!("Directory does not exist: {}", normalized_path));
    }
    if !path_buf.is_dir() {
        return Err(format!("Path is not a directory: {}", normalized_path));
    }

    Ok(normalized_path)
}

/// Prompts user for a yes/no response
fn prompt_yes_no(prompt: &str) -> Result<bool, String> {
    loop {
        print!("{} (y/n): ", prompt);
        io::stdout().flush().map_err(|e| e.to_string())?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("Please enter 'y' or 'n'"),
        }
    }
}

/// Generates TOML configuration content from setup wizard results
fn generate_toml_config(wizard_result: &SetupWizardResult) -> String {
    let mut toml_content = String::new();
    
    toml_content.push_str("# QueryGGUF Configuration File\n\n");
    
    // Add llama-cli path (now using the full path directly)
    toml_content.push_str(&format!("llama_cli_path = \"{}\"\n\n", 
        wizard_result.llama_cpp_directory));
    
    // Add logging configuration
    toml_content.push_str(&format!("logging_enabled = {}\n", wizard_result.logging_enabled));
    if wizard_result.logging_enabled {
        toml_content.push_str(&format!("log_directory_path = \"{}\"\n\n", 
            wizard_result.log_directory_path));
    }

    // Add model directories
    for (i, path) in wizard_result.gguf_model_directories.iter().enumerate() {
        toml_content.push_str(&format!("gguf_model_directory_{} = \"{}\"\n", i + 1, path));
    }
    toml_content.push_str("\n");

    // Add prompt directories
    for (i, path) in wizard_result.prompt_file_directories.iter().enumerate() {
        toml_content.push_str(&format!("prompt_file_directory_{} = \"{}\"\n", i + 1, path));
    }
    
    // Add prompt directory
    toml_content.push_str(&format!("prompt_directory = \"prompts\"\n\n"));

    // Add commented examples for future reference
    toml_content.push_str("# Configuration Examples:\n");
    toml_content.push_str("# Additional model directories can be added as:\n");
    toml_content.push_str("# gguf_model_directory_2 = \"/path/to/more/models\"\n");
    toml_content.push_str("# gguf_model_directory_3 = \"/another/path/to/models\"\n\n");
    
    toml_content.push_str("# Additional prompt directories can be added as:\n");
    toml_content.push_str("# prompt_directory_2 = \"/path/to/more/prompts\"\n");
    toml_content.push_str("# prompt_directory_3 = \"/another/path/to/prompts\"\n\n");
    
    toml_content.push_str("# example llama.cpp llama-cli path:\n");
    toml_content.push_str("# llama_cli_path = \"/home/oopsy/llama.cpp/build/bin/llama-cli\"\n");
   
    
    
    toml_content.push_str("# Saved modes will appear as:\n");
    toml_content.push_str("# mode_1 = \"model_path|prompt_path|temp=0.8|top_k=40|description\"\n\n");


    toml_content
}

/// Saves the configuration to a TOML file in the application's base directory
/// 
/// # Arguments
/// * `config_content` - The TOML configuration content to write to file
/// 
/// # Returns
/// * `Result<(), String>` - Success or error message
/// 
fn save_query_gguf_config(config_content: &str) -> Result<(), String> {
    let config_path = get_config_path()?;
    fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to save configuration: {}", e))?;
    println!("Configuration saved to: {}", config_path.display());
    Ok(())
}

/// Validates that the essential directories in the configuration are accessible
/// Returns Result with () for success or String for error message
fn validate_query_gguf_directories(wizard_result: &SetupWizardResult) -> Result<(), String> {
    // Check model directories
    for path in &wizard_result.gguf_model_directories {
        let path_buf = PathBuf::from(path);
        if !path_buf.exists() || !path_buf.is_dir() {
            return Err(format!("Invalid model directory path: {}", path));
        }
        
        // Check if directory contains any .gguf files
        let has_gguf = fs::read_dir(&path_buf)
            .map_err(|e| format!("Failed to read directory {}: {}", path, e))?
            .any(|entry| {
                entry.ok()
                    .map(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("gguf"))
                    .unwrap_or(false)
            });
        
        if !has_gguf {
            println!("Warning: No .gguf files found in directory: {}", path);
        }
    }

    // Check prompt directories if any exist
    for path in &wizard_result.prompt_file_directories {
        let path_buf = PathBuf::from(path);
        if !path_buf.exists() || !path_buf.is_dir() {
            return Err(format!("Invalid prompt directory path: {}", path));
        }
    }

    // Check log directory if logging is enabled
    if wizard_result.logging_enabled {
        let log_path = PathBuf::from(&wizard_result.log_directory_path);
        if !log_path.exists() || !log_path.is_dir() {
            return Err(format!("Invalid log directory path: {}", 
                wizard_result.log_directory_path));
        }
        
        // Test write permissions on log directory
        let test_file_path = log_path.join("query_gguf_write_test.tmp");
        if let Err(e) = fs::write(&test_file_path, "") {
            return Err(format!("Cannot write to log directory: {}", e));
        }
        let _ = fs::remove_file(test_file_path);
    }

    Ok(())
}

/// Creates a backup of an existing configuration file
/// 
/// Copies the config file to a timestamped backup in the same directory:
/// From: ~/query_gguf/query_gguf_config.toml
/// To:   ~/query_gguf/query_gguf_config_TIMESTAMP.toml.bak
/// 
/// # Returns
/// - Ok(()): Backup created successfully
/// - Err(String): Error message if backup fails
/// 
/// # Error Cases
/// - Source config file not found
/// - Unable to create backup (permissions/disk space)
/// - Path resolution fails
/// 
fn backup_existing_config() -> Result<(), String> {
    // CHANGE 1: Get absolute path to current config
    let config_path = get_config_path()?;

    // CHANGE 2: Only proceed if config exists
    if !config_path.exists() {
        return Ok(());  // No config to backup
    }

    // CHANGE 3: Create backup path in same directory
    let timestamp = generate_timestamp_string();
    let backup_path = config_path.with_file_name(
        format!("query_gguf_config_{}.toml.bak", timestamp)
    );

    // CHANGE 4: Copy file using absolute paths
    fs::copy(&config_path, &backup_path)
        .map_err(|e| format!("Failed to create backup: {}", e))?;

    println!("Created backup of existing config: {}", backup_path.display());
    Ok(())
}

/// Main function to handle the setup process
fn handle_query_gguf_setup() -> Result<(), String> {
    if query_gguf_config_exists() {
        println!("\nExisting Query-GGUF configuration found.");
        match prompt_yes_no("Do you want to create a new configuration?") {
            Ok(true) => {
                backup_existing_config()
                    .map_err(|e| format!("Failed to backup existing config: {}", e))?;
            }
            Ok(false) => {
                println!("Keeping existing configuration.");
                return Ok(());
            }
            Err(e) => return Err(format!("Error during prompt: {}", e)),
        }
    }

    // Create prompts directory and blank prompt file first
    println!("Creating initial prompt directory and blank prompt file...");
    create_blank_prompt()?;

    let wizard_result = run_query_gguf_setup_wizard()?;
    
    // Validate directories before saving
    validate_query_gguf_directories(&wizard_result)?;

    let config_content = generate_toml_config(&wizard_result);
    save_query_gguf_config(&config_content)
        .map_err(|e| format!("Failed to save configuration: {}", e))?;

    println!("\nQuery-GGUF configuration completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_toml_config() {
        let test_result = SetupWizardResult {
            gguf_model_directories: vec!["/path/to/models".to_string()],
            prompt_file_directories: vec!["/path/to/prompts".to_string()],
            log_directory_path: "/path/to/logs".to_string(),
            logging_enabled: true,
            llama_cpp_directory: "/path/to/llama-cli".to_string(), // Added this line
        };

        let config = generate_toml_config(&test_result);
        
        assert!(config.contains("logging_enabled = true"));
        assert!(config.contains("/path/to/models"));
        assert!(config.contains("/path/to/prompts"));
        assert!(config.contains("/path/to/logs"));
        assert!(config.contains("/path/to/llama-cli")); // Added this check
    }

    #[test]
    fn test_directory_validation() {
        let temp_dir = std::env::temp_dir();
        let result = SetupWizardResult {
            gguf_model_directories: vec![temp_dir.to_str().unwrap().to_string()],
            prompt_file_directories: vec![],
            log_directory_path: temp_dir.to_str().unwrap().to_string(),
            logging_enabled: true,
            llama_cpp_directory: temp_dir.join("llama-cli")  // Added this line
                .to_string_lossy()
                .to_string(),
        };

        assert!(validate_query_gguf_directories(&result).is_ok());
    }
}

/// old
/// The function reads a single line from a TOML file that starts with a specified field name
/// and ends with a value. The function returns an empty string if the field is not found, and
/// does not panic or unwrap in case of errors. The function uses only standard Rust libraries
/// and does not introduce unnecessary dependencies.
///
/// design:
/// 0. start with an empty string to return by default
/// 1. get file at path
/// 2. open as text
/// 3. iterate through rows
/// 4. look for filed name as start of string the " = "
/// 5. grab that whole row of text
/// 6. remove "fieldname = " from the beginning
/// 7. remove '" ' and trailing spaces from the end
/// 8. return that string, if any
/// by default, return an empty string, if anything goes wrong, 
/// handle the error, and return an empty string
///
/// requires:
/// use std::fs::File;
/// use std::io::{self, BufRead};
///
/// example use:
///     let value = read_field_from_toml("test.toml", "fieldname");
///
/// new
/// The function reads a single line from a TOML file that starts with a specified field name.
/// The file path is obtained using get_config_path() to ensure the correct absolute path.
/// The function returns an empty string if the field is not found, and
/// does not panic or unwrap in case of errors.
///
/// # Arguments
/// * `field_name` - The name of the field to search for in the TOML file
///
/// # Returns
/// * `String` - The value of the field if found, empty string otherwise
///
/// # Examples
/// ```
/// let llama_path = read_field_from_toml("llama_cli_path");
/// if llama_path.is_empty() {
///     println!("llama_cli_path not found in config");
/// }
/// ```
fn read_field_from_toml(field_name: &str) -> String {
    // Get absolute path to config file
    let path = match get_config_path() {
        Ok(path) => path,
        Err(e) => {
            println!("Error read_field_from_toml getting config path: {}", e);
            return String::new();
        }
    };
    
    // Validate input parameters
    // A PathBuf is invalid if it has no file name component
    if path.file_name().is_none() || field_name.is_empty() {
        println!("Error: read_field_from_toml Invalid path or empty field name provided");
        return String::new();
    }

    // New check:
    if !path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .map_or(false, |ext| ext == "toml") 
    {
        println!("Warning: read_field_from_toml File does not have .toml extension: {}", path.display());

    }

    // Debug print statement
    println!("Attempting read_field_from_toml to open file at path: {}", path.display());


    // Open the file at the specified path
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(e) => {
            // More detailed error reporting
            println!("Failed read_field_from_toml to open file at path: {}. Error: {}", path.display(), e);
            return String::new();
        },
    };

    // Debug print statement
    println!("read_field_from_toml Successfully opened file at path: {}", path.display());


    // Create a buffered reader to read the file line by line
    let reader = io::BufReader::new(file);

    // Keep track of line numbers for better error reporting
    let mut line_number = 0;

    // Iterate through each line in the file
    for line_result in reader.lines() {
        line_number += 1;

        // Handle line reading errors
        let line = match line_result {
            Ok(line) => line,
            Err(e) => {
                println!("Error read_field_from_toml reading line {}: {}", line_number, e);
                continue;
            }
        };

        // Skip empty lines and comments
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }

        // Debug print statement
        println!("Processing line {}: {}", line_number, line);

        // Check if line starts with field name
        if line.trim_start().starts_with(field_name) {
            // Debug print statement
            println!("Found field '{}' on line {}", field_name, line_number);

            // Split the line by '=' and handle malformed lines
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                println!("Malformed TOML line {} - missing '=': {}", line_number, line);
                continue;
            }

            let key = parts[0].trim();
            let value = parts[1].trim();

            // Verify exact field name match (avoiding partial matches)
            if key != field_name {
                continue;
            }

            // Handle empty values
            if value.is_empty() {
                println!("Warning: Empty value found for field '{}'", field_name);
                return String::new();
            }

            // Debug print statement
            println!("Extracted value: {}", value);

            // Clean up the value: remove quotes and trim spaces
            let cleaned_value = value.trim().trim_matches('"').trim();
            
            // Verify the cleaned value isn't empty
            if cleaned_value.is_empty() {
                println!("Warning: Value became empty after cleaning for field '{}'", field_name);
                return String::new();
            }

            return cleaned_value.to_string();
        }
    }

    // If we get here, the field wasn't found
    println!("Field '{}' not found in file", field_name);
    String::new()
}

/// Reads all fields from a TOML file that share a common base name (prefix before underscore)
/// and returns a vector of their values.
/// 
/// Uses the standard config file location:
/// - Linux/MacOS: ~/query_gguf/query_gguf_config.toml
/// - Windows: \Users\username\query_gguf\query_gguf_config.toml
///
/// # Arguments
/// * `base_name` - Base name to search for (e.g., "prompt" will match "prompt_1", "prompt_2", etc.)
///
/// # Returns
/// * `Vec<String>` - Vector containing all values for fields matching the base name
///
fn read_basename_fields_from_toml(base_name: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut numbered_values = Vec::new();  // Store (number, value) pairs

    // Get config path
    let path = match get_config_path() {
        Ok(path) => path,
        Err(e) => {
            println!("Failed to get config path: {}", e);
            return values;
        }
    };

    // Validate input parameters
    if base_name.is_empty() {
        println!("Error: Empty base name provided");
        return values;
    }

    // // Open and read the file
    // let file = match File::open(&path) {
    //     Ok(file) => file,
    //     Err(e) => {
    //         println!("Failed to open file at path: {}. Error: {}", path.display(), e);
    //         return values;
    //     },
    // };

    // let reader = io::BufReader::new(file);
    // let base_name_with_underscore = format!("{}_", base_name);

    // Open and read the file
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to open file at path: {}. Error: {}", path.display(), e);
            return values;
        },
    };

    let reader = io::BufReader::new(file);
    let base_name_with_underscore = format!("{}_", base_name);

    for (line_number, line_result) in reader.lines().enumerate() {
        let line = match line_result {
            Ok(line) => line,
            Err(e) => {
                println!("Error reading line {}: {}", line_number + 1, e);
                continue;
            }
        };

        // Skip empty lines and comments
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
            continue;
        }

        // Check if line starts with base_name_
        if trimmed_line.starts_with(&base_name_with_underscore) {
            // Extract the number after the underscore
            if let Some(num_str) = trimmed_line
                .split('=')
                .next()
                .and_then(|s| s.trim().strip_prefix(&base_name_with_underscore))
            {
                if let Ok(num) = num_str.parse::<usize>() {
                    // Split the line by '=' and handle malformed lines
                    let parts: Vec<&str> = trimmed_line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let value = parts[1].trim().trim_matches('"').trim();
                        if !value.is_empty() {
                            numbered_values.push((num, value.to_string()));
                        }
                    }
                }
            }
        }
    }

    // Sort by the actual mode numbers
    numbered_values.sort_by_key(|(num, _)| *num);
    
    // Extract just the values in correct order
    values = numbered_values.into_iter().map(|(_, value)| value).collect();

    values
}

/// Defines all adjustable parameters for the llama.cpp command execution
/// Each field corresponds to a specific llama.cpp command line argument
#[derive(Debug, Clone)]
struct LlamaCppParameters {
    temperature_value: f32,      // --temp parameter
    top_k_sampling: i32,         // --top-k parameter
    top_p_sampling: f32,         // --top-p parameter
    context_size: i32,           // --ctx-size parameter
    thread_count: i32,           // --threads parameter
    gpu_layers: i32,             // --n-gpu-layers parameter
    interactive_first: bool,     // --interactive-first flag
}
    
    // temperature_value: f32,      // --temp parameter
    // top_k_sampling: i32,         // --top-k parameter
    // top_p_sampling: f32,         // --top-p parameter
    // min_p_sampling: f32,         // --min-p parameter
    // random_seed: i32,            // --seed parameter
    // tail_free_sampling: f32,     // --tfs parameter
    // thread_count: i32,           // --threads parameter
    // typical_sampling: f32,       // --typical parameter
    // mirostat_version: i32,       // --mirostat parameter
    // mirostat_learning_rate: f32, // --mirostat-lr parameter
    // mirostat_entropy: f32,       // --mirostat-ent parameter
    // context_window_size: i32,    // --ctx-size parameter
// }

impl Default for LlamaCppParameters {
    fn default() -> Self {
        Self {
            temperature_value: 0.8,
            top_k_sampling: 40,
            top_p_sampling: 0.9,
            context_size: 2000,
            thread_count: get_system_cpu_count(),
            gpu_layers: 0,       // default to CPU-only
            interactive_first: true,
        }
        // Self {
        //     temperature_value: 0.8,
        //     top_k_sampling: 40,
        //     top_p_sampling: 0.9,
        //     min_p_sampling: 0.05,
        //     random_seed: -1,
        //     tail_free_sampling: 1.0,
        //     thread_count: get_system_cpu_count() - 1,
        //     typical_sampling: 1.0,
        //     mirostat_version: 2,
        //     mirostat_learning_rate: 0.05,
        //     mirostat_entropy: 3.0,
        //     context_window_size: 500,
        // }
    }
}

/// Retrieves the number of CPU cores available on the current system minus 1
/// Returns the number of available CPU cores minus 1 or a safe default if detection fails
fn get_system_cpu_count() -> i32 {
    match std::thread::available_parallelism() {
        Ok(count) => {
            let cpu_count = count.get() as i32;
            // Ensure we don't return less than 1 thread
            if cpu_count > 1 {
                cpu_count - 1
            } else {
                1
            }
        },
        Err(_) => {
            println!("Warning: Could not detect CPU count, using default value of 3");
            3 // conservative default (assuming at least 4 cores)
        }
    }
}

/// Generates a unique timestamp string for log file names and entries
/// Returns a string representation of the current Unix timestamp
fn generate_timestamp_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => {
            println!("Warning: System time error, using 'unknown_time' as timestamp");
            "unknown_time".to_string()
        }
    }
}

/// Creates a blank prompt file in the prompts directory
/// 
/// Creates the file 'blankprompt.txt' in the standard prompts directory:
/// - Linux/MacOS: ~/query_gguf/prompts/blankprompt.txt
/// - Windows: \Users\username\query_gguf\prompts\blankprompt.txt
/// 
/// This blank prompt serves as a default when no specific prompt is selected.
/// The function ensures both the prompts directory and the blank prompt file exist.
/// 
/// # Returns
/// - Ok(String): Absolute path to the created blank prompt file
/// - Err(String): Error message if creation fails
/// 
/// # Error Cases
/// - Cannot create prompts directory (permissions/disk space)
/// - Cannot create blank prompt file
/// - Path resolution fails
/// 
fn create_blank_prompt() -> Result<String, String> {
    // CHANGE 1: Get absolute path to prompts directory
    let prompts_dir = get_prompts_dir()?;
    let blank_prompt_path = prompts_dir.join("blankprompt.txt");

    println!("DEBUG: Creating prompt directory: {}", prompts_dir.display());
    
    // CHANGE 2: Create prompts directory with all parent directories
    fs::create_dir_all(&prompts_dir)
        .map_err(|e| format!("Failed to create prompts directory: {}", e))?;

    println!("DEBUG: Creating blank prompt file: {}", blank_prompt_path.display());
    
    // CHANGE 3: Create the blank prompt file with minimal content
    fs::write(&blank_prompt_path, "# Blank prompt file\n")
        .map_err(|e| format!("Failed to create blank prompt file: {}", e))?;

    // CHANGE 4: Verify the file was created
    if !blank_prompt_path.exists() {
        return Err("Failed to verify blank prompt file creation".to_string());
    }

    println!("Successfully created blank prompt file at: {}", blank_prompt_path.display());
    Ok(blank_prompt_path.to_string_lossy().to_string())
}

/// Handles prompt directory setup, creating a default if needed
fn setup_prompt_directory() -> Result<String, String> {
    println!("\nPrompt Directory Setup:");
    println!("Prompts are text files that will be used to start conversations with LLaMA.");
    
    let prompts_dir = match prompt_yes_no("Do you already have a directory containing prompt files?") {
        Ok(true) => {
            prompt_for_directory("Enter the path to your existing prompts directory")?
        },
        Ok(false) => {
            // Create default prompts directory in current working directory
            let default_prompts_dir = "prompts";
            println!("DEBUG: Creating default prompts directory: {}", default_prompts_dir);
            fs::create_dir_all(default_prompts_dir)
                .map_err(|e| format!("Failed to create prompts directory: {}", e))?;
            
            println!("\nCreated new prompts directory: {}/", default_prompts_dir);
            println!("You can add your prompt text files here.");
            default_prompts_dir.to_string()
        },
        Err(e) => return Err(format!("Error during prompt: {}", e))
    };

    // Always create blankprompt.txt
    println!("\nCreating blank prompt file...");
    let blank_prompt_path = create_blank_prompt()?;
    
    // Verify the file exists
    if !Path::new(&blank_prompt_path).exists() {
        return Err(format!("Failed to verify blank prompt file exists at: {}", blank_prompt_path));
    }

    // Print current directory and file listing for debugging
    println!("DEBUG: Current directory: {:?}", std::env::current_dir().unwrap_or_default());
    println!("DEBUG: Contents of prompts directory:");
    if let Ok(entries) = fs::read_dir("prompts") {
        for entry in entries {
            if let Ok(entry) = entry {
                println!("  {:?}", entry.path());
            }
        }
    }

    Ok(prompts_dir)
}

/// Sets up the logging directory, using default or custom path
fn setup_log_directory() -> Result<String, String> {
    let default_log_dir = "query_gguf/chatlogs";
    
    println!("\nLog Directory Setup:");
    println!("Chat logs will be saved in: {}/", default_log_dir);
    
    match prompt_yes_no("Would you like to use a different directory for logs?") {
        Ok(true) => {
            prompt_for_directory("Enter custom path for log files")
        },
        Ok(false) => {
            // Create default log directory
            match fs::create_dir_all(default_log_dir) {
                Ok(()) => {
                    println!("Using default log directory: {}/", default_log_dir);
                    Ok(default_log_dir.to_string())
                },
                Err(e) => Err(format!("Failed to create log directory: {}", e))
            }
        },
        Err(e) => Err(format!("Error during prompt: {}", e))
    }
}

fn launch_llama(mode: &ChatModeConfig) -> Result<(), String> {
    let llama_cli_path = read_field_from_toml("llama_cli_path");
    if llama_cli_path.is_empty() {
        return Err("LLaMA CLI path not found in configuration".to_string());
    }

    // Construct the llama-cli command string
    let mut llama_command = format!("\"{}\" -m \"{}\"", llama_cli_path, mode.model_path);
    
    // Add prompt file (now always present)
    llama_command.push_str(&format!(" --file \"{}\"", mode.prompt_path));

    // Add all parameters
    llama_command.push_str(&format!(" --temp {}", mode.parameters.temperature_value));
    llama_command.push_str(&format!(" --top-k {}", mode.parameters.top_k_sampling));
    llama_command.push_str(&format!(" --top-p {}", mode.parameters.top_p_sampling));
    llama_command.push_str(&format!(" --ctx-size {}", mode.parameters.context_size));
    llama_command.push_str(&format!(" --threads {}", mode.parameters.thread_count));

    if mode.parameters.gpu_layers > 0 {
        llama_command.push_str(&format!(" --n-gpu-layers {}", mode.parameters.gpu_layers));
    }

    if mode.parameters.interactive_first {
        llama_command.push_str(" --interactive-first");
    }

    llama_command.push_str(" --no-display-prompt");

    println!("\nPreparing to launch LLaMA.cpp gguf llama-cli in a new terminal...");
    println!("Command: {}", llama_command);

    // Launch in new terminal based on OS
    let launch_result = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "start", "cmd", "/K", &llama_command])
            .status()
            .map_err(|e| format!("Failed to launch Windows terminal: {}", e))
    } else if cfg!(target_os = "linux") {
        // Try different terminal emulators
        let terminals = ["xterm", "gnome-terminal", "konsole", "xfce4-terminal"];
        let mut last_error = String::from("No terminal emulator found");

        for terminal in terminals.iter() {
            let result = if *terminal == "gnome-terminal" {
                Command::new(terminal)
                    .args(&["--", "bash", "-c", &format!("{};read -p 'Press Enter to close...'", llama_command)])
                    .status()
            } else {
                Command::new(terminal)
                    .args(&["-e", &format!("bash -c '{};read -p \"Press Enter to close...\"'", llama_command)])
                    .status()
            };

            match result {
                Ok(_) => return Ok(()),
                Err(e) => last_error = format!("Failed to launch {}: {}", terminal, e),
            }
        }
        
        Err(last_error)
    } else if cfg!(target_os = "macos") {
        Command::new("osascript")
            .args(&["-e", &format!(
                "tell application \"Terminal\" to do script \"{}\"",
                llama_command
            )])
            .status()
            .map_err(|e| format!("Failed to launch macOS terminal: {}", e))
    } else {
        Err(String::from("Unsupported operating system"))
    };

    match launch_result {
        Ok(_) => {
            println!("LLaMA launched in new terminal window");
            Ok(())
        },
        Err(e) => Err(format!("Failed to launch LLaMA: {}", e))
    }
}

fn handle_mode_selection(choice: &str) -> Result<String, String> {
    match choice.trim() {
        "dir" | "directory" => {
            println!("\nDirectory Mode Setup:");
            
            // Get directory to scan
            print!("Enter directory path to scan: ");
            io::stdout().flush().map_err(|e| e.to_string())?;
            let dir_path = read_user_input()?.trim().to_string();
            
            // Get mode number to use
            print!("Enter mode number to use: ");
            io::stdout().flush().map_err(|e| e.to_string())?;
            let mode_num = read_user_input()?.trim().to_string();
            
            // Get the selected mode
            let saved_modes = read_saved_modes()?;
            let mode_index = mode_num.parse::<usize>()
                .map_err(|_| "Invalid mode number".to_string())?
                .checked_sub(1)
                .ok_or("Invalid mode number".to_string())?;
            
            let mut selected_mode = saved_modes.get(mode_index)
                .ok_or("Invalid mode selection")?
                .clone();  // Now clones the entire ChatModeConfig

            // Create combined prompt
            let combined_prompt_path = create_combined_prompt(
                &selected_mode.prompt_path,
                &dir_path
            )?;

            // Update mode to use combined prompt
            selected_mode.prompt_path = combined_prompt_path;

            // Launch with combined prompt
            launch_llama(&selected_mode)?;

            Ok(format!("directory_mode::{}", selected_mode.name))
        },
        "make" | "manual" => handle_manual_mode_selection(),
        number => {
            let mode_num = number.parse::<usize>()
                .map_err(|_| "Invalid mode number".to_string())?;

            let saved_modes = read_saved_modes()?;
            
            // Directly use the mode number (1-based index)
            let mode_index = mode_num - 1;
            
            if let Some(mode) = saved_modes.get(mode_index) {
                println!("\nSelected saved mode: {}", mode.name);
                println!("Model: {}", mode.model_path);
                println!("Prompt: {}", mode.prompt_path); // Now always present
                println!("Parameters:");
                display_parameters(&mode.parameters);
                
                println!("\nLaunching LLaMA...");
                launch_llama(mode)?;
                
                Ok(format!("saved_mode::{}", mode.name))
            } else {
                Err("Invalid mode selection".to_string())
            }
        },
    }
}

/// Clears the terminal screen in a cross-platform way
fn clear_screen() {
    if cfg!(windows) {
        let _ = Command::new("cmd").arg("/c").arg("cls").status();
    } else {
        let _ = Command::new("clear").status();
    }
}

/// Reads a line of user input
fn read_user_input() -> Result<String, String> {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read input: {}", e))?;
    Ok(input)
}

/// Represents a model file with its path and name
struct ModelFile {
    full_path: String,
    display_name: String,
}

/// Guides the user through creating a new chat mode configuration
/// 
/// This interactive process:
/// 1. Lists available GGUF models from configured directories
/// 2. Allows model selection
/// 3. Offers prompt file selection
/// 4. Enables parameter configuration
/// 5. Provides option to save as a named mode
/// 
/// File paths are handled using standard locations:
/// - Models: Read from directories in ~/query_gguf/query_gguf_config.toml
/// - Prompts: ~/query_gguf/prompts/
/// - Config: ~/query_gguf/query_gguf_config.toml
/// 
/// # Returns
/// - Ok(String): Success message with format "manual::{model_name}"
/// - Err(String): Error message if any step fails
/// 
/// # Error Cases
/// - No models found
/// - Invalid model selection
/// - Prompt file access fails
/// - Parameter configuration fails
/// - Save operation fails
/// 
/// # Example Success Return
/// ```
/// Ok("manual::llama-7b-q4")
/// ```
/// 
/// # File Path Handling
/// - Uses absolute paths for reliability
/// - Expands home directory (~) in paths
/// - Validates file existence before operations
/// Handles the manual mode selection process
fn handle_manual_mode_selection() -> Result<String, String> {
    // clear_screen();
    println!("\n=== Manual Mode Setup ===");

    // 1. Find and list available models
    let models = find_gguf_models()?;
    if models.is_empty() {
        return Err("No GGUF models found in configured directories".to_string());
    }

    println!("\nAvailable Models:");
    for (index, model) in models.iter().enumerate() {
        println!("{}. {}", index + 1, model.display_name);
    }

    // 2. Get model selection
    print!("\nSelect model number: ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let model_choice = read_user_input()?;
    let model_index = model_choice.trim().parse::<usize>()
        .map_err(|_| "Invalid model number".to_string())?
        .checked_sub(1)
        .ok_or("Invalid model number".to_string())?;

    let selected_model = models.get(model_index)
        .ok_or("Invalid model selection".to_string())?;

    // 3. Handle prompt selection
    let prompt_path = if prompt_yes_no("Would you like to use a prompt file?")? {
        select_prompt_file()?
    } else {
        // Use blank prompt when no prompt is selected
        get_prompts_dir()?.join("blankprompt.txt").to_string_lossy().to_string()
    };

    // 4. Configure parameters
    let parameters = configure_model_parameters()?;

    // 5. Create launch configuration
    let launch_config = LaunchConfiguration {
        model_path: selected_model.full_path.clone(),
        prompt_path,
        parameters,
    };

    // 6. Offer to save as mode
    offer_to_save_mode(&launch_config)?;

    Ok(format!("manual::{}", selected_model.display_name))
}

/// Finds all GGUF model files in the configured model directories
/// 
/// Reads the configuration file from the standard location:
/// - Linux/MacOS: ~/query_gguf/query_gguf_config.toml
/// - Windows: \Users\username\query_gguf\query_gguf_config.toml
/// 
/// Searches all directories listed as gguf_model_directory_* entries in the config,
/// including their subdirectories, for files with .gguf extension.
/// 
/// # Returns
/// - Ok(Vec<ModelFile>): List of found model files with their paths and names
/// - Err(String): Error message if config cannot be read or directories cannot be accessed
/// 
/// # Path Handling
/// - Uses absolute paths for reliability
/// - Expands home directory (~) in paths
/// - Maintains both full path and display name for each model
/// 
/// # Error Cases
/// - Config file not found
/// - Cannot read config file
/// - Model directories don't exist
/// - Insufficient permissions
/// 
/// # Example Config Entries
/// ```toml
/// gguf_model_directory_1 = "/home/user/models"
/// gguf_model_directory_2 = "~/alternative/models"
/// ```
fn find_gguf_models() -> Result<Vec<ModelFile>, String> {
    // Get absolute path to config file
    let config_path = get_config_path()?;
    
    // Read config file
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config at {}: {}", config_path.display(), e))?;

    let mut models = Vec::new();
    let home_dir = get_home_dir()?;

    // Parse config file line by line to find model directories
    for line in config_content.lines() {
        if line.starts_with("gguf_model_directory_") {
            if let Some(path) = line.split('=').nth(1) {
                let raw_path = path.trim().trim_matches('"');
                
                // Resolve path to absolute, handling ~ expansion
                let base_path = if raw_path.starts_with('~') {
                    format!("{}{}", home_dir, &raw_path[1..])
                } else if !Path::new(raw_path).is_absolute() {
                    format!("{}/{}", home_dir, raw_path)
                } else {
                    raw_path.to_string()
                };

                println!("Searching for models in: {}", base_path);
                search_directory_for_gguf(&mut models, Path::new(&base_path))?;
            }
        }
    }

    if models.is_empty() {
        println!("\nWarning: No .gguf files found in configured directories or their subdirectories.");
    } else {
        models.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        println!("Found {} model files", models.len());
    }

    Ok(models)
}

/// Recursively searches a directory and its subdirectories for .gguf files
fn search_directory_for_gguf(models: &mut Vec<ModelFile>, dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Err(format!("Directory does not exist: {}", dir.display()));
    }

    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_dir() {
                            // Recursively search subdirectories
                            let _ = search_directory_for_gguf(models, &path);
                        } else if path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                            // Found a .gguf file
                            println!("Found model: {}", path.display());
                            models.push(ModelFile {
                                full_path: path.to_string_lossy().to_string(),
                                display_name: path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                            });
                        }
                    }
                    Err(e) => println!("Warning: Error reading directory entry: {}", e),
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to read directory {}: {}", dir.display(), e))
    }
}

/// Provides interactive prompt file selection from the standard prompts directory
/// 
/// Lists available prompt files from:
/// - Linux/MacOS: ~/query_gguf/prompts/
/// - Windows: \Users\username\query_gguf\prompts\
/// 
/// This function:
/// 1. Lists all available prompt files with numbers
/// 2. Allows user selection by number
/// 3. Returns absolute path to selected prompt
/// 
/// # Returns
/// - Ok(String): Absolute path to selected prompt file
/// - Err(String): Error message if:
///   - No prompt files found
///   - Invalid selection
///   - File access errors
/// 
/// # Path Handling
/// - Uses absolute paths for reliability
/// - Validates file existence before returning
/// - Maintains consistent path format across OS
/// 
/// # Example Success Path
/// ```
/// "/home/username/query_gguf/prompts/system_prompt.txt"
/// ```
/// 
/// # Error Cases
/// - Empty prompts directory
/// - Invalid number entered
/// - Number out of range
/// - Selected file no longer exists
fn select_prompt_file() -> Result<String, String> {
    // Get all prompt files
    let prompts = find_prompt_files()?;
    
    if prompts.is_empty() {
        return Err("No prompt files found in configured directories".to_string());
    }

    println!("\nAvailable Prompts:");
    // Display prompts with cleaner names
    for (index, prompt) in prompts.iter().enumerate() {
        let path = Path::new(prompt);
        let display_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| prompt.as_str());
        println!("{}. {} ({})", index + 1, display_name, path.display());
    }

    print!("\nSelect prompt number (1-{}): ", prompts.len());
    io::stdout().flush().map_err(|e| format!("Failed to flush output: {}", e))?;
    
    let choice = read_user_input()?;
    let index = choice.trim().parse::<usize>()
        .map_err(|_| "Please enter a valid number".to_string())?
        .checked_sub(1)
        .ok_or("Please enter a number greater than 0".to_string())?;

    if index >= prompts.len() {
        return Err(format!("Please enter a number between 1 and {}", prompts.len()));
    }

    // Get the selected prompt path
    let selected_prompt = &prompts[index];
    
    // Verify the path and convert to absolute
    let absolute_path = Path::new(selected_prompt).canonicalize()
        .map_err(|e| format!("Failed to resolve prompt path: {}", e))?;

    // Verify file still exists
    if !absolute_path.exists() {
        return Err("Selected prompt file no longer exists".to_string());
    }

    // Log the selection
    println!("Selected prompt: {}", absolute_path.display());

    Ok(absolute_path.to_string_lossy().to_string())
}
    
/// Finds all prompt files in the configured prompts directory
/// 
/// This function:
/// 1. Gets the absolute path to the standard prompts directory
/// 2. Creates the directory if it doesn't exist
/// 3. Recursively searches for all files in that directory
/// 4. Returns paths as absolute paths for reliability
/// 
/// Standard Location:
/// - Linux/MacOS: ~/query_gguf/prompts/
/// - Windows: \Users\username\query_gguf\prompts\
/// 
/// # Returns
/// - Ok(Vec<String>): List of absolute paths to found prompt files
/// - Err(String): Error message if directory cannot be accessed or created
/// 
/// # Error Cases
/// - Home directory cannot be determined
/// - Insufficient permissions to create/access directory
/// - IO errors while reading directory contents
/// 
/// # Example Usage
/// ```
/// match find_prompt_files() {
///     Ok(prompts) => {
///         for prompt in prompts {
///             println!("Found prompt: {}", prompt);
///         }
///     },
///     Err(e) => println!("Error finding prompts: {}", e)
/// }
/// ```
fn find_prompt_files() -> Result<Vec<String>, String> {
    // Get absolute path to prompts directory
    let prompts_dir = get_prompts_dir()?;
    
    println!("Searching for prompts in: {}", prompts_dir.display());
    
    let mut prompts = Vec::new();
    
    // Search the directory recursively
    search_directory_for_prompts(&mut prompts, &prompts_dir)?;

    if prompts.is_empty() {
        println!("\nNotice: No prompt files found in directory: {}", prompts_dir.display());
        println!("You can add prompt files to this directory at any time.");
    } else {
        prompts.sort();
        println!("Found {} prompt files", prompts.len());
    }

    Ok(prompts)
}

/// Recursively searches a directory and its subdirectories for prompt files
/// 
/// This function:
/// 1. Creates the directory if it doesn't exist
/// 2. Recursively searches the directory and all subdirectories
/// 3. Adds all found files to the prompts vector
/// 4. Stores paths as absolute paths
/// 
/// # Arguments
/// * `prompts` - Vector to store found prompt file paths
/// * `dir` - Directory to search
/// 
/// # Returns
/// - Ok(()): Search completed successfully
/// - Err(String): Error message if directory cannot be accessed
/// 
/// # Error Cases
/// - Directory creation fails
/// - Insufficient permissions
/// - IO errors while reading directory
/// 
fn search_directory_for_prompts(prompts: &mut Vec<String>, dir: &Path) -> Result<(), String> {
    // Create directory if it doesn't exist
    if !dir.exists() {
        fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create directory {}: {}", dir.display(), e))?;
        println!("Created directory: {}", dir.display());
        return Ok(());
    }

    // Read directory contents
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

    // Process each entry
    for entry_result in entries {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                
                if path.is_dir() {
                    // Recursively search subdirectories
                    if let Err(e) = search_directory_for_prompts(prompts, &path) {
                        println!("Warning: Error searching subdirectory {}: {}", path.display(), e);
                    }
                } else {
                    // Convert path to absolute if it isn't already
                    match path.canonicalize() {
                        Ok(abs_path) => {
                            println!("Found prompt file: {}", abs_path.display());
                            prompts.push(abs_path.to_string_lossy().to_string());
                        },
                        Err(e) => println!("Warning: Could not resolve path {}: {}", path.display(), e)
                    }
                }
            },
            Err(e) => println!("Warning: Error reading directory entry: {}", e),
        }
    }

    Ok(())
}

/// Reads and parses all saved chat modes from the configuration file
/// 
/// This function:
/// 1. Gets the absolute path to the config file in the user's home directory
/// 2. Reads all mode_* entries from the config file
/// 3. Parses each mode entry into a ChatModeConfig struct
/// 
/// Config file location:
/// - Linux/MacOS: ~/query_gguf/query_gguf_config.toml
/// - Windows: \Users\username\query_gguf\query_gguf_config.toml
/// 
/// Mode entries in config should be formatted as:
/// mode_1 = "model_path|prompt_path|param=value|param=value|name|description"
/// 
/// # Returns
/// - Ok(Vec<ChatModeConfig>): Vector of parsed chat modes
/// - Err(String): Error message if config cannot be read or parsed
/// 
/// # Example Config Entry
/// ```toml
/// mode_1 = "/path/to/model.gguf|prompts/system.txt|temp=0.8|top_k=40|FastMode|Quick responses"
/// ```
/// 
/// # Field Order
/// 1. model_path (required)
/// 2. prompt_path (required)
/// 3. parameters (optional, format: name=value)
/// 4. mode name (required)
/// 5. description (required)
/// 
/// # Error Cases
/// - Config file not found
/// - Invalid mode format
/// - Missing required fields
/// 
fn read_saved_modes() -> Result<Vec<ChatModeConfig>, String> {
    // let config_path = get_config_path()?;
    let mode_fields = read_basename_fields_from_toml("mode");
    let mut modes = Vec::new();

    // Get base directories once at the start
    let home_dir = get_home_dir()?;
    let prompts_dir = get_prompts_dir()?;
    
    for (index, config_str) in mode_fields.iter().enumerate() {
        let parts: Vec<&str> = config_str.split('|').collect();
        if parts.len() < 2 {
            println!("Warning: Skipping malformed mode entry {}: insufficient parts", index + 1);
            continue;
        }

        // 1. CHANGE: Resolve model path to absolute path
        let model_path = if Path::new(parts[0]).is_absolute() {
            parts[0].to_string()
        } else {
            format!("{}/{}", home_dir, parts[0].trim_start_matches("/"))
        };
        println!("Resolved model path: {}", model_path);

        // 2. CHANGE: Resolve prompt path to absolute path
        let prompt_path = if parts.len() > 1 && !parts[1].contains('=') {
            if Path::new(parts[1]).is_absolute() {
                parts[1].to_string()
            } else {
                // Strip any leading "prompts/" from the path before joining
                let clean_path = parts[1]
                    .trim_start_matches("prompts/")
                    .trim_start_matches('/');
                prompts_dir.join(clean_path)
                    .to_string_lossy()
                    .to_string()
            }
        } else {
            // 3. CHANGE: Use absolute path for default blank prompt
            prompts_dir.join("blankprompt.txt")
                .to_string_lossy()
                .to_string()
        };
        println!("Resolved prompt path: {}", prompt_path);

        // Get the last two non-parameter parts for name and description
        let mut name = String::new();
        let mut description = String::new();
            
        // Find the last two non-parameter parts
        let non_param_parts: Vec<&str> = parts.iter()
            .filter(|&&part| !part.contains('='))
            .cloned()
            .collect();
            
        if non_param_parts.len() >= 2 {
            name = non_param_parts[non_param_parts.len() - 2].to_string();
            description = non_param_parts[non_param_parts.len() - 1].to_string();
        } else {
            println!("Warning: Mode {} missing name or description", index + 1);
        }

        let parameters = parse_parameters_from_parts(&parts);

        let mode_config = ChatModeConfig {
            name,
            description,
            model_path,
            prompt_path,
            parameters,
        };
        modes.push(mode_config);
    }

    if modes.is_empty() {
        println!("Warning: No valid modes found in config file");
    }

    Ok(modes)
}

/// Parses parameters from mode configuration parts
fn parse_parameters_from_parts(parts: &[&str]) -> LlamaCppParameters {
    let mut params = LlamaCppParameters::default();

    for part in parts {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "temp" => if let Ok(v) = value.parse() { params.temperature_value = v },
                "top_k" => if let Ok(v) = value.parse() { params.top_k_sampling = v },
                "top_p" => if let Ok(v) = value.parse() { params.top_p_sampling = v },
                "ctx_size" => if let Ok(v) = value.parse() { params.context_size = v },
                "threads" => if let Ok(v) = value.parse() { 
                    params.thread_count = validate_thread_count(v) 
                },
                "gpu_layers" => if let Ok(v) = value.parse() { params.gpu_layers = v },
                "interactive_first" => if let Ok(v) = value.parse() { params.interactive_first = v },
                _ => (), // Ignore unknown parameters
            }
        }
    }

    params
}

/// Configuration for launching LLaMA
struct LaunchConfiguration {
    model_path: String,
    prompt_path: String,
    parameters: LlamaCppParameters,
}

/// Allows user to configure model parameters with option to skip
fn configure_model_parameters() -> Result<LlamaCppParameters, String> {
    let mut params = LlamaCppParameters::default();
    
    println!("\nModel Parameters:");
    match prompt_yes_no("Would you like to modify default parameters?") {
        Ok(false) => {
            println!("Using default parameters:");
            display_parameters(&params);
            return Ok(params);
        },
        Ok(true) => {
            println!("\nEnter new values (or press Enter to keep default):");
            configure_parameters_interactive(&mut params)?;
        },
        Err(e) => return Err(e),
    }

    println!("\nFinal parameter configuration:");
    display_parameters(&params);
    Ok(params)
}

fn display_parameters(params: &LlamaCppParameters) {
    // Remove the if let Some(prompt) check since prompt_path is now always present
    println!("  Temperature: {}", params.temperature_value);
    println!("  Top-K: {}", params.top_k_sampling);
    println!("  Top-P: {}", params.top_p_sampling);
    println!("  Context Size: {}", params.context_size);
    println!("  Threads: {}", params.thread_count);
    println!("  GPU Layers: {}", params.gpu_layers);
    println!("  Interactive First: {}", params.interactive_first);
}

/// Validates and adjusts thread count to ensure it's within reasonable bounds
fn validate_thread_count(threads: i32) -> i32 {
    let max_threads = get_system_cpu_count() + 1; // Allow up to actual CPU count
    let min_threads = 1;
    
    if threads < min_threads {
        println!("Warning: Thread count too low, using minimum of {}", min_threads);
        min_threads
    } else if threads > max_threads {
        println!("Warning: Thread count exceeds CPU count, using maximum of {}", max_threads);
        max_threads
    } else {
        threads
    }
}

/// Interactively configure parameters
fn configure_parameters_interactive(params: &mut LlamaCppParameters) -> Result<(), String> {
    // Temperature
    print!("Temperature (default {}): ", params.temperature_value);
    io::stdout().flush().map_err(|e| e.to_string())?;
    if let Ok(input) = read_user_input() {
        if !input.trim().is_empty() {
            params.temperature_value = input.trim().parse()
                .map_err(|_| "Invalid temperature value".to_string())?;
        }
    }

    // Top-K
    print!("Top-K sampling (default {}): ", params.top_k_sampling);
    io::stdout().flush().map_err(|e| e.to_string())?;
    if let Ok(input) = read_user_input() {
        if !input.trim().is_empty() {
            params.top_k_sampling = input.trim().parse()
                .map_err(|_| "Invalid Top-K value".to_string())?;
        }
    }

    // Top-P
    print!("Top-P sampling (default {}): ", params.top_p_sampling);
    io::stdout().flush().map_err(|e| e.to_string())?;
    if let Ok(input) = read_user_input() {
        if !input.trim().is_empty() {
            params.top_p_sampling = input.trim().parse()
                .map_err(|_| "Invalid Top-P value".to_string())?;
        }
    }

    // Context Size
    print!("Context window size (default {}): ", params.context_size);
    io::stdout().flush().map_err(|e| e.to_string())?;
    if let Ok(input) = read_user_input() {
        if !input.trim().is_empty() {
            params.context_size = input.trim().parse()
                .map_err(|_| "Invalid context size value".to_string())?;
        }
    }

    // Thread Count
    print!("Thread count (default: auto-detected {} [CPU count - 1]): ", params.thread_count);
    io::stdout().flush().map_err(|e| e.to_string())?;
    if let Ok(input) = read_user_input() {
        if !input.trim().is_empty() {
            params.thread_count = input.trim().parse()
                .map_err(|_| "Invalid thread count".to_string())?;
        }
    }

    // GPU Layers
    print!("Number of GPU layers (0 for CPU-only, default {}): ", params.gpu_layers);
    io::stdout().flush().map_err(|e| e.to_string())?;
    if let Ok(input) = read_user_input() {
        if !input.trim().is_empty() {
            params.gpu_layers = input.trim().parse()
                .map_err(|_| "Invalid GPU layers value".to_string())?;
        }
    }

    // Interactive First
    params.interactive_first = prompt_yes_no("Enable interactive-first mode?")?;

    Ok(())
}

#[derive(Debug, Clone)]
struct ChatModeConfig {
    name: String,
    description: String,
    model_path: String,
    prompt_path: String,
    parameters: LlamaCppParameters,
}

/// Offers to save the current configuration as a new mode
fn offer_to_save_mode(config: &LaunchConfiguration) -> Result<(), String> {
    if prompt_yes_no("\nWould you like to save this configuration as a named mode?")? {
        println!("\n=== Save Mode Configuration ===");
        
        // Get mode name
        print!("Enter a name for this mode: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mode_name = read_user_input()?.trim().to_string();
        
        if mode_name.is_empty() {
            return Err("Mode name cannot be empty".to_string());
        }

        // Get mode description
        print!("Enter a brief description for this mode: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let description = read_user_input()?.trim().to_string();

        let new_mode = ChatModeConfig {
            name: mode_name.clone(),
            description,
            model_path: config.model_path.clone(),
            prompt_path: config.prompt_path.clone(),
            parameters: config.parameters.clone(),
        };

        save_mode_to_config(&new_mode)?;
        println!("\nMode '{}' saved successfully!", mode_name);
    }
    Ok(())
}

/// Saves a new chat mode configuration to the config file
/// 
/// Writes to standard config location:
/// - Linux/MacOS: ~/query_gguf/query_gguf_config.toml
/// - Windows: \Users\username\query_gguf\query_gguf_config.toml
/// 
/// This function:
/// 1. Reads existing configuration
/// 2. Counts existing modes
/// 3. Optionally sets as default mode
/// 4. Formats and appends new mode entry
/// 5. Saves updated configuration
/// 
/// # Arguments
/// * `mode` - ChatModeConfig containing all mode settings
/// 
/// # Returns
/// - Ok(()): Mode saved successfully
/// - Err(String): Error message if save fails
/// 
/// # Format
/// Saves modes in format:
/// ```toml
/// # Mode N - name - description
/// mode_N = "model_path|prompt_path|params...|name|description"
/// ```
/// 
/// # Error Cases
/// - Config file not found
/// - Permission denied
/// - Disk full
/// - IO errors
fn save_mode_to_config(mode: &ChatModeConfig) -> Result<(), String> {
    // let config_path = "query_gguf_config.toml";
    let config_path = get_config_path()?;
    
    // Read existing config
    // let mut config_content = fs::read_to_string(config_path)
    //     .map_err(|e| format!("Failed to read config: {}", e))?;
    let mut config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config at {}: {}", config_path.display(), e))?;
    
    // Count existing modes
    let mode_count = config_content.lines()
        .filter(|line| line.starts_with("mode_"))
        .count();
    let new_mode_num = mode_count + 1;

    // Ask if this should be the default mode
    if prompt_yes_no("Would you like to make this the default mode?")? {
        // Remove existing default_mode line if it exists
        config_content = config_content.lines()
            .filter(|line| !line.starts_with("default_mode"))
            .collect::<Vec<&str>>()
            .join("\n");
        
        // Add new default_mode line
        config_content.push_str(&format!("\ndefault_mode = {}\n", new_mode_num));
    }
    
    // Format new mode entry with comment showing name and description
    let mut new_mode_entry = format!("\n# Mode {} - {} - {}\n", 
        new_mode_num, 
        mode.name,
        mode.description
    );
    
    // Start the mode entry with the model path and prompt path (now always present)
    new_mode_entry.push_str(&format!("mode_{} = \"{}|{}",
        new_mode_num, 
        mode.model_path,
        mode.prompt_path
    ));
    
    // Add parameters
    new_mode_entry.push_str(&format!("|temp={}|top_k={}|top_p={}|ctx_size={}|threads={}|gpu_layers={}|interactive_first={}",
        mode.parameters.temperature_value,
        mode.parameters.top_k_sampling,
        mode.parameters.top_p_sampling,
        mode.parameters.context_size,
        mode.parameters.thread_count,
        mode.parameters.gpu_layers,
        mode.parameters.interactive_first,
    ));
    
    // Add name and description at the end
    new_mode_entry.push_str(&format!("|{}|{}\"\n", mode.name, mode.description));

    // Append to config file
    config_content.push_str(&new_mode_entry);
    // fs::write(config_path, config_content)
    //     .map_err(|e| format!("Failed to write config: {}", e))?;
    fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to write config to {}: {}", config_path.display(), e))?;
    Ok(())
}

/// Displays the available modes in a simplified format
fn display_available_modes() {
    println!("\nQuery-GGUF - Select a mode number or type a command:");
    println!("Commands:");
    println!("  'make' or 'manual' -> Create new mode");
    println!("  'dir' or 'directory' -> Run with directory contents");
    println!("  'config' -> Open config file in editor");

    println!("\nAvailable Modes:");
    match read_saved_modes() {
        Ok(modes) => {
            for (index, mode) in modes.iter().enumerate() {
                println!("{}. {} - {}", 
                    index + 1, 
                    mode.name,        // Display the actual name
                    mode.description  // Display the actual description
                );
            }
        }
        Err(e) => {
            println!("Warning: Could not read saved modes: {}", e);
        }
    }
}

/// Opens the configuration file in the system's text editor
/// 
/// Editor selection priority:
/// 1. $EDITOR environment variable if set
/// 2. Platform-specific default:
///    - Windows: notepad
///    - Linux/MacOS: nano
/// 
/// Opens the config file at standard location:
/// - Linux/MacOS: ~/query_gguf/query_gguf_config.toml
/// - Windows: \Users\username\query_gguf\query_gguf_config.toml
/// 
/// # Returns
/// - Ok(()): Editor opened and config edited successfully
/// - Err(String): Error message if:
///   - Config path cannot be resolved
///   - Editor cannot be launched
///   - Editor process fails
/// 
/// # Platform Handling
/// - Uses appropriate default editor per OS
/// - Handles path differences between platforms
/// - Maintains consistent config location
/// 
/// # Error Cases
/// - Config file not found
/// - Editor not available
/// - Insufficient permissions
/// - Process spawn failure
fn open_config_in_editor() -> Result<(), String> {
    // Get absolute path to config file
    let config_path = get_config_path()?;
    
    // Verify config exists
    if !config_path.exists() {
        return Err(format!("Configuration file not found at: {}", config_path.display()));
    }

    // Select appropriate default editor based on platform
    let default_editor = if cfg!(windows) {
        "notepad"
    } else {
        "nano"
    };

    // Get editor from environment or use default
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| default_editor.to_string());

    println!("Opening config with editor: {}", editor);
    println!("Config path: {}", config_path.display());

    // Launch editor with absolute config path
    let status = Command::new(&editor)
        .arg(config_path.as_os_str())
        .spawn()
        .map_err(|e| format!("Failed to launch editor '{}': {}", editor, e))?
        .wait()
        .map_err(|e| format!("Error while editing with '{}': {}", editor, e))?;

    // Check if editor exited successfully
    if !status.success() {
        return Err(format!("Editor '{}' exited with error status", editor));
    }

    println!("Configuration file edited successfully");
    Ok(())
}

/// Represents a directory scan result
struct DirectoryScan {
    tree_structure: String,
    file_contents: String,
}

// /// Recursively scans a directory and builds a tree-like structure with file contents
// fn scan_directory(path: &Path, prefix: &str) -> Result<DirectoryScan, String> {
//     let mut tree = String::new();
//     let mut contents = String::new();

//     if !path.exists() {
//         return Err(format!("Directory not found: {}", path.display()));
//     }

//     let entries = fs::read_dir(path)
//         .map_err(|e| format!("Failed to read directory {}: {}", path.display(), e))?;

//     // Sort entries for consistent output
//     let mut entries: Vec<_> = entries.collect::<Result<Vec<_>, _>>()
//         .map_err(|e| format!("Failed to collect directory entries: {}", e))?;
//     entries.sort_by_key(|entry| entry.path());

//     for (i, entry) in entries.iter().enumerate() {
//         let is_last = i == entries.len() - 1;
//         let path = entry.path();
//         let name = path.file_name()
//             .and_then(|n| n.to_str())
//             .unwrap_or("invalid_filename");

//         // Add to tree structure
//         tree.push_str(&format!("{}{} {}\n", 
//             prefix,
//             if is_last { "└──" } else { "├──" },
//             name));

//         if path.is_dir() {
//             // Recursively scan subdirectory
//             let next_prefix = format!("{}{}",
//                 prefix,
//                 if is_last { "    " } else { "│   " });
            
//             let scan_result = scan_directory(&path, &next_prefix)?;
//             tree.push_str(&scan_result.tree_structure);
//             contents.push_str(&scan_result.file_contents);
//         } else {
//             // Read file contents if it's a text file
//             if let Ok(file_type) = infer::get_from_path(&path) {
//                 if let Some(mime) = file_type {
//                     if mime.mime_type().starts_with("text/") {
//                         if let Ok(content) = fs::read_to_string(&path) {
//                             contents.push_str(&format!("\n=== {} ===\n{}\n", name, content));
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     Ok(DirectoryScan {
//         tree_structure: tree,
//         file_contents: contents,
//     })
// }

/// Checks if a file might be a text file based on extension
fn is_likely_text_file(path: &Path) -> bool {
    let text_extensions = [
        "txt", "md", "rs", "py", "js", "json", "toml", "yaml", "yml",
        "css", "html", "htm", "xml", "csv", "log", "sh", "bash",
        "c", "cpp", "h", "hpp", "java", "go", "rb", "pl", "php"
    ];
    
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| text_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Recursively scans a directory and builds a tree-like structure with file contents
fn scan_directory(path: &Path, prefix: &str) -> Result<DirectoryScan, String> {
    let mut tree = String::new();
    let mut contents = String::new();

    if !path.exists() {
        return Err(format!("Directory not found: {}", path.display()));
    }

    let entries = fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory {}: {}", path.display(), e))?;

    // Sort entries for consistent output
    let mut entries: Vec<_> = entries.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect directory entries: {}", e))?;
    entries.sort_by_key(|entry| entry.path());

    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == entries.len() - 1;
        let path = entry.path();
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("invalid_filename");

        // Add to tree structure
        tree.push_str(&format!("{}{} {}\n", 
            prefix,
            if is_last { "└──" } else { "├──" },
            name));

        if path.is_dir() {
            // Recursively scan subdirectory
            let next_prefix = format!("{}{}",
                prefix,
                if is_last { "    " } else { "│   " });
            
            let scan_result = scan_directory(&path, &next_prefix)?;
            tree.push_str(&scan_result.tree_structure);
            contents.push_str(&scan_result.file_contents);
        } else {
            // Read file contents if it's a text file
            if is_likely_text_file(&path) {
                if let Ok(content) = fs::read_to_string(&path) {
                    contents.push_str(&format!("\n=== {} ===\n{}\n", name, content));
                }
            }
        }
    }

    Ok(DirectoryScan {
        tree_structure: tree,
        file_contents: contents,
    })
}

/// Creates a combined prompt file with directory contents
fn create_combined_prompt(
    original_prompt_path: &str,
    directory_path: &str
) -> Result<String, String> {
    // Get the prompts directory
    let prompts_dir = get_prompts_dir()?;
    
    // Generate timestamp for unique filename
    let timestamp = generate_timestamp_string();
    let combined_prompt_path = prompts_dir
        .join(format!("combined_prompt_{}.txt", timestamp));

    // Read original prompt
    let original_prompt = fs::read_to_string(original_prompt_path)
        .map_err(|e| format!("Failed to read original prompt: {}", e))?;

    // Scan directory
    let scan_result = scan_directory(
        Path::new(directory_path), 
        ""
    )?;

    // Combine prompts
    let combined_content = format!(
        "{}\n\nDirectory Structure:\n{}\n\nFile Contents:{}\n",
        original_prompt,
        scan_result.tree_structure,
        scan_result.file_contents
    );

    // Write combined prompt
    fs::write(&combined_prompt_path, combined_content)
        .map_err(|e| format!("Failed to write combined prompt: {}", e))?;

    Ok(combined_prompt_path.to_string_lossy().to_string())
}



/// Modified mode selection screen for simpler interaction
fn display_mode_selection_screen() -> Result<String, String> {
    loop {
        display_available_modes();

        print!("\nEnter selection: ");
        io::stdout().flush().map_err(|e| e.to_string())?;

        let choice = read_user_input()?.trim().to_lowercase();
        
        match choice.as_str() {
            "" => {
                // Handle empty input - try to use default mode
                let default_mode = read_field_from_toml("default_mode");
                if !default_mode.is_empty() {
                    if let Ok(mode_num) = default_mode.parse::<usize>() {
                        return handle_mode_selection(&mode_num.to_string());
                    }
                }
                println!("\nNo default mode set. Please make a selection.");
                continue;
            },
            "quit" | "q" | "exit" => {
                return Err("User requested exit".to_string());
            },
            "config" => {
                open_config_in_editor()?;
                continue;
            },
            "make" | "manual" => {
                return handle_manual_mode_selection();
            },
            "dir" | "directory" => {
                return handle_mode_selection("dir");
            },
            number => {
                // Try to parse as a mode number
                if let Ok(mode_num) = number.parse::<usize>() {
                    match handle_mode_selection(&mode_num.to_string()) {
                        Ok(mode) => return Ok(mode),
                        Err(e) => {
                            println!("\nError: {}", e);
                            println!("Press Enter to continue...");
                            let _ = read_user_input()?;
                            continue;
                        }
                    }
                } else {
                    println!("\nInvalid selection. Press Enter to continue...");
                    let _ = read_user_input()?;
                    continue;
                }
            }
        }
    }
}

/// Handles quick launch by checking for command line arguments
fn handle_quick_launch() -> Result<(), String> {
    // Only check for command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        // Use the first argument as mode selection
        handle_mode_selection(&args[1])?;
        return Ok(());
    }

    // If no command line arguments, return Ok to continue to interactive mode
    Ok(())
}

/// Modified main function for cleaner flow
fn main() -> Result<(), String> {
    println!("Query via gguf llama.cpp llama-cli");

    // Check if we need to run setup
    if !query_gguf_config_exists() {
        println!("\nNo configuration found. Starting setup...");
        handle_query_gguf_setup()?;
        println!("\nSetup completed. Press Enter to continue...");
        read_user_input()?;
    }

    // Try quick launch first
    match handle_quick_launch() {
        Ok(()) => {
            // Quick launch succeeded or wasn't available
            // Show mode selection screen if quick launch didn't handle it
            match display_mode_selection_screen() {
                Ok(_mode) => Ok(()),
                Err(e) if e == "User requested exit" => {
                    println!("Goodbye!");
                    Ok(())
                },
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Err(e)
                }
            }
        },
        Err(e) => {
            eprintln!("Quick launch error: {}", e);
            Err(e)
        }
    }
}
