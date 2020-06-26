# fixparser

![](https://github.com/whoan/fixparser/workflows/build-and-test/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/fixparser.svg)](https://crates.io/crates/fixparser)
[![Docs.rs](https://docs.rs/fixparser/badge.svg)](https://docs.rs/fixparser)

Parse FIX messages without a FIX dictionary.

```
[dependencies]
fixparser = "0.1.2"
```

It currently supports the following input/output formats:

**Input:**

- [FIX Tag=Value (classic FIX)](https://www.fixtrading.org/standards/tagvalue/)

**Output:**

- Json (`serde_json::value::Value`)

## Examples

```rust
let input = "Recv | 8=FIX.4.4 | 555=2 | 600=CGY | 604=2 | 605=F7 | 605=CGYU0 | 600=CGY | 10=209";

if let Some(fix_message) = fixparser::FixMessage::from_tag_value(&input) {
    println!("{}", fix_message.to_json());
}
```

```rust
// this input has the non-printable character 0x01 as the separator of the fields
let input = "8=FIX.4.4555=2600=CGY604=2605=F7605=CGYU0600=CGY10=209";
if let Some(fix_message) = fixparser::FixMessage::from_tag_value(&input) {
    println!("{}", fix_message.to_json());
}
```

For any of those examples you will have this output:

```
{"8":"FIX.4.4","555":[{"600":"CGY","604":[{"605":"F7"},{"605":"CGYU0"}]},{"600":"CGY"}],"10":"209"}
```

Or prettier (`jq`'ed):

```
{
  "8": "FIX.4.4",
  "555": [
    {
      "600": "CGY",
      "604": [
        {
          "605": "F7"
        },
        {
          "605": "CGYU0"
        }
      ]
    },
    {
      "600": "CGY"
    }
  ],
  "10": "209"
}
```

## Goodies

- It supports groups and you don't need a FIX dictionary
- You don't need to specify the separator of the input string as long as they are consistent. eg: 0x01, |, etc...
- You don't need to "trim" the input string as the lib detects the beginning and end of the message

## Limitations

- There is scenario the library might parse the message incorrectly as it can't guess the format of the message without a dictionary:

```
8=FIX.4.4 | 1000=2 | 1001=1 | 1002=2 | 1001=10 | 1002=20 | 1003=30 | 10=209
              ^                                              ^
          group 1000                does 1003 belong to the second repetition of group 1000?
```

In such scenario, it will assume *1003* does not belong to the group.

## Features

You can debug the library using the `debugging` feature:

```
fixparser = { version = "<version>", features = ["debugging"] }
```

## License

[MIT](https://github.com/whoan/fixparser/blob/master/LICENSE)
