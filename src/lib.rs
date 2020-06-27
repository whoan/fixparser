//! `fixparser` is a Rust library to decode FIX (Financial Information eXchange) messages.
//!
//! - It supports groups and you don't need a FIX dictionary
//! - You don't need to specify the separator of the input string as long as they are consistent. eg: 0x01, |, etc...
//! - You don't need to "trim" the input string as the lib detects the beginning and end of the message
//!
//! Currently supported input:
//!
//! - [FIX Tag=Value (classic FIX)](https://www.fixtrading.org/standards/tagvalue/)
//!
//! Currently supported output:
//!
//! - Json (serde_json::value::Value)

extern crate regex;

use serde::{ser::SerializeMap, Serialize, Serializer};
use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(feature = "debugging")]
macro_rules! debug {
    ($($arg:tt)*) => { println!($($arg)*); }
}

#[cfg(not(feature = "debugging"))]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

#[derive(Debug, Clone)]
enum FixEntity {
    Field(i32, String),
    Group(FixGroup),
}

impl FixEntity {
    fn get_tag(&self) -> i32 {
        match self {
            FixEntity::Field(tag, _dummy) => *tag,
            FixEntity::Group(group) => group.no_tag,
        }
    }

    fn get_field_value_i32(&self) -> i32 {
        if let FixEntity::Field(_dummy, value) = self {
            return value.parse().unwrap();
        }
        panic!("A Field was expected");
    }
}

#[derive(Debug, Clone)]
struct FixComponent {
    entities: Vec<FixEntity>,
}

impl FixComponent {
    fn new(entities: Vec<FixEntity>) -> Self {
        Self { entities }
    }
}

impl Serialize for FixComponent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.entities.len()))?;
        for entity in &self.entities {
            match entity {
                FixEntity::Field(ref tag, ref value) => {
                    map.serialize_entry(tag, value)?;
                }
                FixEntity::Group(ref group) => {
                    map.serialize_entry(&group.no_tag, &group.instances)?;
                }
            }
        }
        map.end()
    }
}

#[derive(Debug, Clone)]
struct FixGroup {
    delimiter: i32, // first tag of each group instance
    no_tag: i32,    // tag which contains the number of repetitions
    repetitions: i32,
    current_iteration: i32,
    known_tags: HashSet<i32>, // tags we know that belong to this group
    instances: Vec<FixComponent>,
}

impl FixGroup {
    fn new(delimiter: i32, index_first_delimiter: usize, component: &mut FixComponent) -> Self {
        let group_instance =
            FixComponent::new(component.entities.drain(index_first_delimiter..).collect());
        let no_tag_field = component.entities.pop().unwrap();

        Self {
            no_tag: no_tag_field.get_tag(), // bad variable name, as in FIX
            delimiter,
            repetitions: no_tag_field.get_field_value_i32(),
            current_iteration: 1,
            known_tags: Self::get_known_tags(&group_instance),
            instances: vec![group_instance],
        }
    }

    fn get_known_tags(group_instance: &FixComponent) -> HashSet<i32> {
        let mut known_tags = HashSet::<i32>::new();
        group_instance
            .entities
            .iter()
            .for_each(|entity| match entity {
                FixEntity::Field(tag, _value) => {
                    known_tags.insert(*tag);
                }
                FixEntity::Group(group) => {
                    group.known_tags.iter().for_each(|known_tag| {
                        known_tags.insert(*known_tag);
                    });
                }
            });
        known_tags
    }

    fn create_new_instance(&mut self) {
        self.instances.push(FixComponent::new(Vec::new()));
    }

    fn insert_known_tag(&mut self, tag: i32) {
        self.known_tags.insert(tag);
    }
}

#[derive(Debug)]
struct TagValue<'a>(i32, &'a str);

/// This is the interface you interact with.
///
/// The internal message is represented as follows:
///
/// ```ignore
/// FixMessage    := FixComponent
/// FixComponent  := FixEntity*
///
/// FixEntity     := Field | Group
///
/// Field         := (tag: i32, value: String)
/// Group         := FixComponent*
/// ```
pub struct FixMessage {
    root_component: FixComponent,
    pending_tag_indices: HashMap<i32, VecDeque<usize>>,
    candidate_indices: Vec<HashMap<i32, usize>>, // store indices of tags of potential nested group
    active_groups: Vec<FixGroup>,                // contains the groups currently being parsed
}

impl FixMessage {
    fn new() -> Self {
        let mut candidate_indices = Vec::new();
        candidate_indices.push(HashMap::new());
        Self {
            root_component: FixComponent::new(Vec::new()),
            pending_tag_indices: HashMap::new(),
            candidate_indices,
            active_groups: Vec::new(),
        }
    }

    /// Creates a FixMessage from an input string encoded in [FIX Tag=Value (classic FIX)](https://www.fixtrading.org/standards/tagvalue/).
    ///
    /// # Examples
    ///
    /// ```rust
    /// let input = "Recv | 8=FIX.4.4 | 555=2 | 600=CGY | 604=2 | 605=F7 | 605=CGYU0 | 600=CGY | 10=209";
    ///
    /// if let Some(fix_message) = fixparser::FixMessage::from_tag_value(&input) {
    ///     println!("{}", fix_message.to_json());
    /// }
    /// ```
    pub fn from_tag_value(input_message: &str) -> Option<FixMessage> {
        let tag_values = FixMessage::pre_process_message(&input_message)?;
        let mut message = FixMessage::new();

        for (index, tag_value) in tag_values.iter().enumerate() {
            message
                .pending_tag_indices
                .entry(tag_value.0)
                .or_insert_with(VecDeque::new)
                .push_back(index);
        }
        message.check_message_is_valid()?;

        for (index, tag_value) in tag_values.iter().enumerate() {
            message.add_tag_value(tag_value.0, String::from(tag_value.1), index);
        }
        message.clean();

        Some(message)
    }

    /// Get a representation of the message in json string format.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // this input has the non-printable character 0x01 as the separator of the fields
    /// let input = "8=FIX.4.4555=2600=CGY604=2605=F7605=CGYU0600=CGY10=209";
    ///
    /// if let Some(fix_message) = fixparser::FixMessage::from_tag_value(&input) {
    ///     println!("{}", fix_message.to_json());
    /// }
    /// ```
    ///
    /// ```ignore
    /// {"8":"FIX.4.4","555":[{"600":"CGY","604":[{"605":"F7"},{"605":"CGYU0"}]},{"600":"CGY"}],"10":"209"}
    /// ```
    pub fn to_json(&self) -> serde_json::value::Value {
        serde_json::json!(&self.root_component)
    }

    // from tag value encoding to a list of TagValue's
    fn pre_process_message<'a>(input_message: &'a str) -> Option<Vec<TagValue<'a>>> {
        if input_message.len() < 16 { // len(8=FIX.N.M|10=123) = 16
            return None
        }
        let start_offset = input_message.find("8=")?;
        let field_separator = Self::get_separator(&input_message[start_offset..])?;
        let mut end_of_message_found = false;

        input_message[start_offset..]
            .split(&field_separator)
            .map(|tag_value| {
                tag_value.split_at(tag_value.find('=').unwrap_or_else(|| tag_value.len()))
            })
            .filter(|tag_value| tag_value.1.len() > 1)
            .map(|tag_value| TagValue(tag_value.0.parse().unwrap_or(0), &tag_value.1[1..]))
            .take_while(|tag_value| {
                if end_of_message_found {
                    eprintln!("WARNING: Detected tag after tag 10: {}", tag_value.0);
                    return false;
                }
                end_of_message_found = tag_value.0 == 10;
                true
            })
            .map(Some)
            .collect()
    }

    // get FIX values separator: eg: 0x01 or |
    fn get_separator(fix_msg: &str) -> Option<String> {
        let fix_version_re = regex::Regex::new(r"^8=FIXT?.\d{1}.\d{1}").unwrap();
        let field_separator = &fix_msg[fix_version_re.shortest_match(fix_msg)?..]
            .chars()
            .take_while(|char| !char.is_digit(10))
            .collect::<String>();

        debug!("separator [{}]", field_separator);
        if field_separator == "" {
            return None;
        }

        Some(field_separator.to_string())
    }

    fn check_message_is_valid(&self) -> Option<()> {
        if self.pending_tag_indices.get(&10).is_none() {
            eprintln!("WARNING: Message is incomplete. Discarding...");
            return None;
        }
        Some(())
    }

    #[allow(unused_variables)]
    fn add_tag_value(&mut self, tag: i32, value: String, index: usize) {
        debug!(
            "{}Index {} - Added {} - {}",
            self.get_spaces(),
            index,
            tag,
            value
        );
        self.remove_pending_tag(tag);

        while self.is_parsing_group() && !self.tag_in_group(tag) {
            self.close_group();
        }

        if self.repeated_candidate(tag) {
            self.open_group(tag);
        }

        if self.is_parsing_group() {
            self.set_known_tag_in_group(tag);
        }

        if self.is_new_iteration(tag) {
            self.create_new_group_instance();
        } else {
            self.register_candidate(tag);
        }

        self.get_entities().push(FixEntity::Field(tag, value));
    }

    fn clean(&mut self) {
        self.pending_tag_indices.clear();
        self.candidate_indices.clear();
        self.active_groups.clear();
    }

    fn open_group(&mut self, group_delimiter: i32) {
        debug!("{}INFO: Group detected", self.get_spaces());
        let group = FixGroup::new(
            group_delimiter,
            self.get_index_of_candidate(group_delimiter),
            self.get_component(),
        );
        self.active_groups.push(group);
        self.candidate_indices.push(HashMap::new());
    }

    fn get_candidates(&self) -> &HashMap<i32, usize> {
        self.candidate_indices.last().unwrap()
    }

    fn get_candidates_mut(&mut self) -> &mut HashMap<i32, usize> {
        self.candidate_indices.last_mut().unwrap()
    }

    fn get_index_of_candidate(&self, tag: i32) -> usize {
        *self.get_candidates().get(&tag).unwrap()
    }

    // must be called before new insertion
    fn register_candidate(&mut self, tag: i32) {
        let candidate_index = self.get_entities().len();
        self.get_candidates_mut().insert(tag, candidate_index);
    }

    fn repeated_candidate(&mut self, tag: i32) -> bool {
        self.get_candidates().contains_key(&tag)
    }

    fn get_next_index_of_pending_tag(&self, tag: i32) -> Option<&usize> {
        self.pending_tag_indices.get(&tag).unwrap().front()
    }

    fn remove_pending_tag(&mut self, tag: i32) {
        self.pending_tag_indices.get_mut(&tag).unwrap().pop_front();
    }

    fn close_group(&mut self) {
        debug!("{}INFO: Stop parsing group\n", self.get_spaces());
        let closed_group = self.active_groups.pop().unwrap();
        self.get_component()
            .entities
            .push(FixEntity::Group(closed_group));
        self.candidate_indices.pop();
    }

    fn is_new_iteration(&self, tag: i32) -> bool {
        self.is_parsing_group() && tag == self.active_group().delimiter
    }

    fn increment_iteration(&mut self) {
        debug!(
            "{}-- repetition {} --",
            self.get_spaces(),
            self.active_group().current_iteration + 1
        );
        self.active_group_mut().current_iteration += 1
    }

    fn get_entities(&mut self) -> &mut Vec<FixEntity> {
        &mut self.get_component().entities
    }

    fn create_new_group_instance(&mut self) {
        self.get_candidates_mut().clear();
        self.increment_iteration();
        self.active_group_mut().create_new_instance();
    }

    #[allow(dead_code)]
    fn get_spaces(&self) -> String {
        " ".repeat(self.active_groups.len() * 2)
    }

    fn set_known_tag_in_group(&mut self, tag: i32) {
        self.active_group_mut().insert_known_tag(tag);
    }

    fn get_component(&mut self) -> &mut FixComponent {
        if self.is_parsing_group() {
            self.active_group_mut().instances.last_mut().unwrap()
        } else {
            &mut self.root_component
        }
    }

    fn is_parsing_group(&self) -> bool {
        !self.active_groups.is_empty()
    }

    fn active_group(&self) -> &FixGroup {
        self.active_groups.last().unwrap()
    }

    fn active_group_mut(&mut self) -> &mut FixGroup {
        self.active_groups.last_mut().unwrap()
    }

    fn tag_in_group(&mut self, tag: i32) -> bool {
        // from cheaper to more expensive check
        !self.is_last_iteration()
            || self.is_known_group_tag(tag)
            || self.pending_tag_in_last_instance()
    }

    fn pending_tag_in_last_instance(&mut self) -> bool {
        self.active_group().known_tags.iter().any(|known_tag| {
            if let Some(tag_index) = self.get_next_index_of_pending_tag(*known_tag) {
                return self.index_belongs_to_current_group(*tag_index);
            }
            false
        })
    }

    fn index_belongs_to_current_group(&self, tag_index: usize) -> bool {
        if let Some(delimiter_index) =
            self.get_next_index_of_pending_tag(self.active_group().delimiter)
        {
            return tag_index < *delimiter_index;
        }
        true
    }

    fn is_known_group_tag(&self, tag: i32) -> bool {
        self.active_group().known_tags.contains(&tag)
    }

    fn is_last_iteration(&self) -> bool {
        self.active_group().current_iteration == self.active_group().repetitions
    }
}
