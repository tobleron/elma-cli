use elma_cli::*;

fn main() {
    let line = "List the files in _stress_testing/_opencode_for_testing/ and identify the primary entry point of this codebase.";
    let path = extract_first_path_from_user_text(line);
    println!("Prompt: {}", line);
    println!("Extracted path: {:?}", path);
}
