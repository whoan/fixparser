# fixparser

![](https://github.com/whoan/fixparser/workflows/build-and-test/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/fixparser.svg)](https://crates.io/crates/fixparser)
[![Docs.rs](https://docs.rs/fixparser/badge.svg)](https://docs.rs/fixparser)

Parse FIX messages without a FIX dictionary.

```
[dependencies]
fixparser = "0.1.4"
```

It currently supports the following input/output formats:

**Input:**

- [FIX Tag=Value (classic FIX)](https://www.fixtrading.org/standards/tagvalue/)

**Output:**

- Json (`serde_json::value::Value`)

## Goal

To have a low-level mechanism to convert FIX messages to something easier to consume by higher-level tools. In such tools, you can combine the output of this library (json) with a FIX dictionary and let your dreams come true :nerd_face:.

## Examples

```rust
let input = "Recv | 8=FIX.4.4 | 555=2 | 600=CGY | 604=2 | 605=F7 | 605=CGYU0 | 600=CGY | 10=209";
println!("{}", fixparser::FixMessage::from_tag_value(&input).unwrap().to_json());
```

```rust
// this input has the non-printable character 0x01 as the separator of the fields
let input = "8=FIX.4.4555=2600=CGY604=2605=F7605=CGYU0600=CGY10=209";
println!("{}", fixparser::FixMessage::from_tag_value(&input).unwrap().to_json());
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

Give it a try:

```bash
cargo run --example from-stdin
```

## Goodies

- It supports repeating groups
- You don't need a FIX dictionary. It is easy to create a tool to combine the output (json) with a dictionary
- You don't need to specify the separator of the input string as long as they are consistent. eg: 0x01, |, etc...
- You don't need to "trim" the input string as the lib detects the beginning and end of the message
- You don't need a delimiter (eg: SOH) in the last field
- It makes minimal validations on the message to allow parsing FIX messages with wrong values

## Features

You can debug the library using the `debugging` feature:

```
fixparser = { version = "<version>", features = ["debugging"] }
```

## Nive-to-have features

- Support [data fields](https://www.onixs.biz/fix-dictionary/5.0.SP2/index.html): data, and XMLData
- Support more [input encodings](https://www.fixtrading.org/standards/)

## Limitations

- There is a scenario where the library needs to make assumptions as it can't guess the format without a dictionary. Example:

```
8=FIX.4.4 | 1000=2 | 1001=1 | 1002=2 | 1001=10 | 1002=20 | 1003=30 | 10=209
              ^                                              ^
          group 1000                does 1003 belong to the second repetition of group 1000?
```

In such a scenario, it will assume *1003* does NOT belong to the group. Doing so, it's easier to fix it with the help of other tools which use FIX dictionaries (coming soon? let's see).

## License

[MIT](https://github.com/whoan/fixparser/blob/master/LICENSE)
