use fixparser::FixMessage;

#[test]
fn minimal_length() {
    let input = "8=FIX.4.4|10=209";
    let output = r#"{"8":"FIX.4.4","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn prefixed() {
    let input = "Recv | 8=FIX.4.4 | 9=something | 10=209";
    let output = r#"{"8":"FIX.4.4","9":"something","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn control_a_separator() {
    let input = "8=FIX.4.4^A9=something^A10=209";
    let output = r#"{"8":"FIX.4.4","9":"something","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn nested_nested_groups_1() {
    let input = "8=FIX.4.4 | 555=2 | 604=2 | 605=F7 | 605=CGYU0 | 604=2 | 605=F7 | 605=CGYM0 | 10=209";
    let output = r#"{"8":"FIX.4.4","555":[{"604":[{"605":"F7"},{"605":"CGYU0"}]},{"604":[{"605":"F7"},{"605":"CGYM0"}]}],"10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn nested_nested_groups_2() {
    let input = "8=FIX.4.4 | 555=2 | 600=CGY | 604=2 | 605=F7 | 605=CGYU0 | 600=CGY | 604=2 | 605=F7 | 605=CGYM0 | 10=209";
    let output = r#"{"8":"FIX.4.4","555":[{"600":"CGY","604":[{"605":"F7"},{"605":"CGYU0"}]},{"600":"CGY","604":[{"605":"F7"},{"605":"CGYM0"}]}],"10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn nested_nested_groups_3() {
    let input = "8=FIX.4.49=0062435=AB49=Sender56=Target34=000003058369=00000005452=20200424-13:54:17.519142=US,NY11=158773500012960=20200424-13:54:17.51848=2D3D22=855=2D3D461=FMMXSX167=FUT555=3600=2D602=1M2MN0603=5608=ACMXSX609=FUT610=202007611=20200730624=49623=1566=3204600=2D602=M2MQ0603=5608=ACMXSX609=FUT610=202008611=20200831624=49623=1566=3204600=2D602=M2MU0603=5608=ACMXSX609=FUT610=202009630=hello631=yes632=it633=works611=20200930624=49623=1566=320444=320438=254=140=277=O59=01028=Y21=110=100";
    let output = r#"{"8":"FIX.4.4","9":"00624","35":"AB","49":"Sender","56":"Target","34":"000003058","369":"000000054","52":"20200424-13:54:17.519","142":"US,NY","11":"1587735000129","60":"20200424-13:54:17.518","48":"2D3D","22":"8","55":"2D3D","461":"FMMXSX","167":"FUT","555":[{"600":"2D","602":"1M2MN0","603":"5","608":"ACMXSX","609":"FUT","610":"202007","611":"20200730","624":"49","623":"1","566":"3204"},{"600":"2D","602":"M2MQ0","603":"5","608":"ACMXSX","609":"FUT","610":"202008","611":"20200831","624":"49","623":"1","566":"3204"},{"600":"2D","602":"M2MU0","603":"5","608":"ACMXSX","609":"FUT","610":"202009","630":"hello","631":"yes","632":"it","633":"works","611":"20200930","624":"49","623":"1","566":"3204"}],"44":"3204","38":"2","54":"1","40":"2","77":"O","59":"0","1028":"Y","21":"1","10":"100"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn more_tags() {
    let input = "8=FIX.4.4 | 10=209 | 11=some";
    let output = r#"{"8":"FIX.4.4","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn value_with_equal() {
    let input = "8=FIX.4.4 | 50=there is an = here | 10=209";
    let output = r#"{"8":"FIX.4.4","50":"there is an = here","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn big_msg() {
    let input = "8=FIX.4.4 | 9=01944 | 35=8 | 49=sender | 56=target | 34=3951 | 50=O001 | 142=US,NY | 52=20200520-19:15:45.134 | 116=john | 129=taylor | 37=07491773 | 198=78652655716 | 526=1589738524192 | 527=07491773-88e4a2169:4 | 11=1589997254902 | 41=19997254901 | 10011=42 | 453=2 | 448=1 | 452=205 | 447=D | 448=FIX_OUT | 452=83 | 447=D | 17=78663 | 150=Z | 18=2 | 39=0 | 1=out | 55=3D | 107=long value here | 460=14 | 48=16735443526687 | 167=MLEG | 762=Strip | 200=202007 | 541=20200701 | 205=1 | 207=IEX | 461=FMMXSX | 15=USD | 54=18765 | 38=10 | 40=2 | 44=2900 | 59=0 | 151=10 | 14=0 | 6=0 | 60=20200520-19:15:45.099000 | 77=O | 442=3 | 1028=N | 582=1 | 21=1 | 454=4 | 455=PA | 456=99 | 455=some-here | 456=98 | 455=3D something | 456=97 | 455=106723 | 456=8 | 555=3 | 600=3D | 620=some long value | 607=14 | 602=168921002590820 | 603=96 | 609=FUT | 610=202007 | 611=20200730 | 616=IEX | 608=FMXSX | 624=1 | 623=1 | 556=USD | 654=1 | 604=5 | 605=PA | 606=99 | 605=2DN0 | 606=98 | 605=3D Jul20 | 606=97 | 605=1M2MN0 | 606=5 | 605=48304 | 606=8 | 600=3D | 620=some long value | 607=14 | 602=1287304730621 | 603=96 | 609=FUT | 610=202008 | 611=20200831 | 616=IEX | 608=FMXSX | 624=1 | 623=1 | 556=USD | 654=2 | 604=5 | 605=PA | 606=99 | 605=2DQ0 | 606=98 | 605=3D Aug20 | 606=97 | 605=1M2MQ0 | 606=5 | 605=48610 | 606=8 | 600=3D | 620=long value | 607=14 | 602=78779119978 | 603=96 | 609=FUT | 610=202009 | 611=20200930 | 616=IEX | 608=FMXSX | 624=1 | 623=1 | 556=USD | 654=3 | 604=5 | 605=PA | 606=99 | 605=2DU0 | 606=98 | 605=3D some | 606=97 | 605=1M2MU0 | 606=5 | 605=45945 | 606=8 | 30=HJGU | 1031=W | 10=139 | ";
    let output = r#"{"8":"FIX.4.4","9":"01944","35":"8","49":"sender","56":"target","34":"3951","50":"O001","142":"US,NY","52":"20200520-19:15:45.134","116":"john","129":"taylor","37":"07491773","198":"78652655716","526":"1589738524192","527":"07491773-88e4a2169:4","11":"1589997254902","41":"19997254901","10011":"42","453":[{"448":"1","452":"205","447":"D"},{"448":"FIX_OUT","452":"83","447":"D"}],"17":"78663","150":"Z","18":"2","39":"0","1":"out","55":"3D","107":"long value here","460":"14","48":"16735443526687","167":"MLEG","762":"Strip","200":"202007","541":"20200701","205":"1","207":"IEX","461":"FMMXSX","15":"USD","54":"18765","38":"10","40":"2","44":"2900","59":"0","151":"10","14":"0","6":"0","60":"20200520-19:15:45.099000","77":"O","442":"3","1028":"N","582":"1","21":"1","454":[{"455":"PA","456":"99"},{"455":"some-here","456":"98"},{"455":"3D something","456":"97"},{"455":"106723","456":"8"}],"555":[{"600":"3D","620":"some long value","607":"14","602":"168921002590820","603":"96","609":"FUT","610":"202007","611":"20200730","616":"IEX","608":"FMXSX","624":"1","623":"1","556":"USD","654":"1","604":[{"605":"PA","606":"99"},{"605":"2DN0","606":"98"},{"605":"3D Jul20","606":"97"},{"605":"1M2MN0","606":"5"},{"605":"48304","606":"8"}]},{"600":"3D","620":"some long value","607":"14","602":"1287304730621","603":"96","609":"FUT","610":"202008","611":"20200831","616":"IEX","608":"FMXSX","624":"1","623":"1","556":"USD","654":"2","604":[{"605":"PA","606":"99"},{"605":"2DQ0","606":"98"},{"605":"3D Aug20","606":"97"},{"605":"1M2MQ0","606":"5"},{"605":"48610","606":"8"}]},{"600":"3D","620":"long value","607":"14","602":"78779119978","603":"96","609":"FUT","610":"202009","611":"20200930","616":"IEX","608":"FMXSX","624":"1","623":"1","556":"USD","654":"3","604":[{"605":"PA","606":"99"},{"605":"2DU0","606":"98"},{"605":"3D some","606":"97"},{"605":"1M2MU0","606":"5"},{"605":"45945","606":"8"}]}],"30":"HJGU","1031":"W","10":"139"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn fix_5_spx() {
    let input = "8=FIXT.1.1 | 10=209";
    let output = r#"{"8":"FIXT.1.1","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn soh_separator() {
    let input = "8=FIX.4.410=209";
    let output = r#"{"8":"FIX.4.4","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

// invalid still parsable messages

#[test]
fn missinig_repeating_group() {
    // WARNING: the lib should generate an output although there is a missing repetition
    let input = "8=FIX.4.4 | 555=3 | 600=QWE | 600=RTY | 10=209";
    let output = r#"{"8":"FIX.4.4","555":[{"600":"QWE"},{"600":"RTY"}],"10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn value_with_separator() {
    // WARNING: anything after a separator in the value of the field, will be truncated
    let input = "8=FIX.4.4 | 50=there is a | here | 10=209";
    let output = r#"{"8":"FIX.4.4","50":"there is a","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn invalid_tag() {
    // WARNING: invalid tags are just ignored together with its value (if any)
    let input = "8=FIX.4.4 | 9=some | thing=wrong | 10=209";
    let output = r#"{"8":"FIX.4.4","9":"some","10":"209"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn missinig_checksum_tag() {
    let input = "8=FIX.4.4 | 9=some";
    let output = r#"{"8":"FIX.4.4","9":"some"}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn missing_checksum_value() {
    let input = "8=FIX.4.4 | 9=some | 10=";
    let output = r#"{"8":"FIX.4.4","9":"some","10":""}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

#[test]
fn shortest_parsable() {
    let input = "8=FIX.4.4|1=";
    let output = r#"{"8":"FIX.4.4","1":""}"#;
    assert_eq!(output, FixMessage::from_tag_value(&input).unwrap().to_json().to_string());
}

// invalid cases from here

#[test]
#[should_panic]
fn too_short() {
    let input = "8=FIX.4.4|1";
    FixMessage::from_tag_value(&input).unwrap().to_json().to_string();
}

#[test]
#[should_panic]
fn missing_fix_version_1() {
    let input = "8= | 10=123";
    FixMessage::from_tag_value(&input).unwrap().to_json().to_string();
}

#[test]
#[should_panic]
fn missing_fix_version_2() {
    let input = "8= | 9=somethinghere | 10=123";
    FixMessage::from_tag_value(&input).unwrap().to_json().to_string();
}
