use bodyfile::Bodyfile3Line;
use std::convert::TryFrom;

fn main() {
    // Create a sample Bodyfile3Line using TryFrom
    let line_str = "0|/test/file|12345|d/rwxr-xr-x|1000|1000|1024|1577836800|1577836800|1577836800|1577836800";
    let line = Bodyfile3Line::try_from(line_str).unwrap();
    
    println!("Bodyfile3Line: {}", line);
    
    // Check the fields by converting back to string
    println!("As string: {}", line.to_string());
}
