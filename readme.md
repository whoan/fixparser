# fix

Decode FIX messages without a FIX dictionary.

```
[dependencies]
fix = "0.1.0"
```

It currently supports the following input/output formats:

**Input:**

- [FIX Tag=Value (classic FIX)](https://www.fixtrading.org/standards/tagvalue/)

**Output:**

- FixComponent (internal representation)
- Json

## Examples

Print internal representation:

```rust
let input = "Recv | 8=FIX.4.4 | 555=2 | 600=CGY | 604=2 | 605=F7 | 605=CGYU0 | 600=CGY | 604=2 | 605=F7 | 605=CGYM0 | 10=209";

if let Some(fix_message) = fix::FixMessage::from_tag_value(&input) {
    println!("{:?}", fix_message.get());
}
```

Print json:

```rust
// this input has the non-printable character 0x01 as the separator of the fields
let input = "8=FIX.4.4555=2600=CGY604=2605=F7605=CGYU0600=CGY604=2605=F7605=CGYM010=20";
if let Some(fix_message) = fix::FixMessage::from_tag_value(&input) {
    println!("{}", serde_json::json!(fix_message.get()).to_string());
}
```

> See tests/ folder for more examples

## Goodies

- It supports groups and you don't need to provide the FIX dictionary
- You don't need to specify the separator of the input string as long as they are consistent. eg: 0x01, |, etc...
- You don't need to "trim" the input string as the lib detects the beginning and end of the message

## Limitations

- There are a few scenarios the library can't parse as it can't guess the format of the message without a dictionary:

```
8=FIX.4.4 | 1000=2 | 1001=1 | 1002=2 | 1001=10 | 1002=20 | 1003=30 | 10=209
              ^                                              ^
          group 1000                does 1003 belong to the second repetition of group 1000?
```

## License

[MIT](https://github.com/whoan/libfix/blob/master/LICENSE)
