use std::{
    io::{stdin, Write},
    thread::sleep,
    time::Duration,
};

fn main() {
    print!(r#"{{"foo":"bar"}}"#);
    std::io::stdout().flush().unwrap();
    sleep(Duration::from_millis(10));
    print!(r#"{{"bar":"baz"}}"#);
    std::io::stdout().flush().unwrap();
    sleep(Duration::from_millis(10));
    print!(r#"{{"baz":123}}"#);
    std::io::stdout().flush().unwrap();

    let mut buffer = String::new();
    stdin().read_line(&mut buffer).unwrap();

    print!("{}", buffer);
    std::io::stdout().flush().unwrap();
}
