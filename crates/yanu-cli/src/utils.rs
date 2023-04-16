use console::style;

pub fn get_section(header: &str, content: &str, pad: &str) -> String {
    let mut section = style(format!("{header}:")).bold().underlined().to_string();
    for line in content.lines() {
        section.push('\n');
        section.push_str(pad);
        section.push_str(line);
    }
    section
}
