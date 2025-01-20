use std::io::IsTerminal;

pub(crate) fn is_interactive() -> bool {
    if cfg!(test) {
        return false;
    }

    std::io::stdin().is_terminal() && 
    std::io::stdout().is_terminal() &&
    !is_ci_environment()
}

pub(crate) fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok() ||
    std::env::var("CONTINUOUS_INTEGRATION").is_ok() ||
    std::env::var("BUILD_NUMBER").is_ok() ||
    std::env::var("GITHUB_ACTIONS").is_ok() ||
    std::env::var("TEAMCITY_VERSION").is_ok() ||
    std::env::var("TRAVIS").is_ok()
}

pub(crate) fn prompt_yes_no(prompt: &str) -> bool {
    println!("{} (y/n)", prompt);
    let mut input = String::new();
    if let Ok(_) = std::io::stdin().read_line(&mut input) {
        input.trim().to_lowercase().starts_with('y')
    } else {
        false
    }
}