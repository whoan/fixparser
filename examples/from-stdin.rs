use std::io::{self, BufRead};

fn main() -> io::Result<()> {
    println!("Give me a tag-value FIX message, and I will give you a JSON.");
    println!("Do you need help? You can try this one: 8=FIX.4.2 | 10=209");

    let stdin = io::stdin();
    let handle = stdin.lock();

    for line in handle.lines() {
        if let Some(fix_message) = fixparser::FixMessage::from_tag_value(&line?) {
            println!("{}", fix_message.to_json());
        } else {
            println!("Are your sure you gave me a valid FIX message?");
        }
    }

    Ok(())
}
