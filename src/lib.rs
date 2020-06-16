use serde::{ser::SerializeMap, Serialize, Serializer};
use std::collections::{HashMap, HashSet, VecDeque};


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

    fn get_value_i32(&self) -> i32 {
        if let FixEntity::Field(_dummy, value) = self {
            return value.parse().unwrap();
        }
        panic!("ill-formated FIX");
    }
}


#[derive(Debug, Clone)]
pub struct FixComponent {
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
    delimiter: i32,
    no_tag: i32,
    repetitions: i32,
    current_iteration: i32,
    instances: Vec<FixComponent>,
    known_tags: HashSet<i32>, // tags we know that belong to this group
    candidate_indices: HashMap<i32, usize>, // store indices of tags in potential nested group <- ?
}

impl FixGroup {
    pub fn new(delimiter: i32, index_first_delimiter: usize, component: &mut FixComponent) -> Self {
        let group_instance =
            FixComponent::new(component.entities.drain(index_first_delimiter..).collect());
        let mut known_tags: HashSet<i32> = group_instance
            .entities
            .iter()
            .map(|fix_entity| fix_entity.get_tag())
            .collect();

        // can I improve this?
        // We need to register as known tags the ones inside a nested repeating group
        for entity in &group_instance.entities {
            if let FixEntity::Group(group) = entity {
                for instance in &group.instances {
                    for entity in &instance.entities {
                        known_tags.insert(entity.get_tag());
                    }
                }
            }
        }

        let group = component.entities.pop().unwrap();
        let no_tag = group.get_tag();  // bad variable name, as in FIX
        let repetitions = group.get_value_i32();
        println!(". Repetitions {}\n", repetitions);

        Self {
            no_tag,
            delimiter,
            repetitions,
            current_iteration: 1,
            instances: vec![group_instance],
            known_tags,
            candidate_indices: HashMap::new(),
        }
    }

    fn insert_known_tag(&mut self, tag: i32) {
        self.known_tags.insert(tag);
    }
}


#[derive(Debug)]
struct TagValue<'a>(i32, &'a str);

pub struct FixMessage {
    pending_tag_indices: HashMap<i32, VecDeque<usize>>,
    candidate_indices: HashMap<i32, usize>,
    pub root_component: FixComponent,
    active_groups: Vec<FixGroup>,
    current_index: usize, // for debugging
    // A, B, no_C=3, C1, C2, C1, no_D=2, D1, D2, D1, D2, C2, C1, C2
    //        ^                   ^
    //   start group C       start group D (in second instance of group C)
}

impl FixMessage {
    fn new() -> Self {
        Self {
            pending_tag_indices: HashMap::new(),
            candidate_indices: HashMap::new(),
            root_component: FixComponent::new(Vec::new()),
            active_groups: Vec::new(),
            current_index: 0,
        }
    }

    pub fn from_raw(fix_message: &str) -> Option<FixMessage> {
        let mut message = FixMessage::new();
        let start_offset = fix_message.find("8=")?;
        let field_separator = Self::get_separator(&fix_message[start_offset..]);
        println!("separator [{}]", field_separator);

        if field_separator == "" {
            return None;
        }

        let mut tag_values: Vec<TagValue> = Vec::new();
        for tag_value in fix_message[start_offset..].split(&field_separator) {
            let tag_value: Vec<&str> = tag_value.split('=').collect();
            if tag_value.len() > 1 {
                let tag = tag_value[0].parse().unwrap_or(0);
                let value = tag_value[1];
                tag_values.push(TagValue(tag, value));
                message
                    .pending_tag_indices
                    .entry(tag)
                    .or_insert(VecDeque::new())
                    .push_back(message.current_index);
                message.current_index += 1;
            }
        }

        let mut end_of_message_found = false;
        message.current_index = 0;
        for tag_value in tag_values.iter() {
            if end_of_message_found {
                println!(
                    "Already processed tag 10. Not processing since: {:?}",
                    tag_value
                );
                break;
            }
            let tag = tag_value.0;
            message.add_tag_value(tag, String::from(tag_value.1));
            println!("{}Index {} - Added {} - {}", message.get_spaces(), message.current_index, tag, tag_value.1);
            message.current_index += 1;
            end_of_message_found = tag_value.0 == 10;
        }
        println!();

        if !end_of_message_found {
            println!("Message processed but incomplete");
        }
        Some(message)
    }

    fn get_separator(fix_msg: &str) -> String {
        let mut index_start: usize = 9; // len(8=FIX.N.M)
        if fix_msg.chars().nth(index_start).unwrap() == '.' {
            index_start += 4; // len(.SPX)
        }
        let mut index_end = index_start;
        for it in fix_msg[index_start..].chars() {
            if it.is_digit(10) {
                break;
            }
            index_end += 1;
        }
        fix_msg[index_start..index_end].to_string()
    }

    fn open_group(&mut self, group_delimiter: i32) {
        print!("{}INFO: Group detected", self.get_spaces());
        let group = FixGroup::new(
            group_delimiter,
            self.get_index_group_delimiter(group_delimiter),
            self.get_component(),
        );
        self.active_groups.push(group);
    }

    fn get_candidates(&mut self) -> &mut HashMap<i32, usize> {
        if self.parsing_group() {
            &mut self.active_group_mut().candidate_indices
        } else {
            &mut self.candidate_indices
        }
    }

    fn get_index_group_delimiter(&mut self, tag: i32) -> usize {
        *self.get_candidates().get(&tag).unwrap()
    }

    fn register_candidate(&mut self, tag: i32, index: usize) {
        self.get_candidates().insert(tag, index);
    }

    fn clear_candidates(&mut self) {
        self.get_candidates().clear();
    }

    fn repeated_tag(&mut self, tag: i32) -> bool {
        self.get_candidates().contains_key(&tag)
    }

    fn remove_pending_tag(&mut self, tag: i32) {
        self.pending_tag_indices.get_mut(&tag).unwrap().pop_front();
    }

    fn close_group(&mut self) {
        println!("{}INFO: Stop parsing group\n", self.get_spaces());
        let closed_group = self.active_groups.pop().unwrap();
        self.get_component().entities.push(FixEntity::Group(closed_group));
    }

    fn add_tag_value(&mut self, tag: i32, value: String) {
        self.remove_pending_tag(tag);

        while self.parsing_group() && !self.tag_in_group(tag) {
            self.close_group();
        }

        if self.repeated_tag(tag) {
            self.open_group(tag);
        }

        if !self.parsing_group() {
            self.root_component
                .entities
                .push(FixEntity::Field(tag, value));
            self.register_candidate(tag, self.root_component.entities.len() - 1);
            return;
        }

        self.set_known_tag_in_group(tag);
        let new_iteration = tag == self.active_group().delimiter;
        if new_iteration {
            self.active_group_mut().current_iteration += 1;
        }

        let group = &mut self.active_group_mut();
        if new_iteration {
            group
                .instances
                .push(FixComponent::new(Vec::new()));
        }
        group
            .instances
            .last_mut()
            .unwrap()
            .entities
            .push(FixEntity::Field(tag, value));
        let index = group.instances.last_mut().unwrap().entities.len() - 1;

        if new_iteration {
            self.clear_candidates();
            println!("{}-- repetition {} --", self.get_spaces(), self.active_group().current_iteration);
        } else {
            self.register_candidate(tag, index);
        }
    }

    fn get_spaces(&self) -> String {
        let mut spaces: Vec<char> = Vec::new();
        spaces.resize(self.active_groups.len() * 2, ' ');
        spaces.iter().collect()
    }

    fn set_known_tag_in_group(&mut self, tag: i32) {
        self.active_group_mut().insert_known_tag(tag);
    }

    fn get_component(&mut self) -> &mut FixComponent {
        if self.parsing_group() {
            self.active_group_mut().instances.last_mut().unwrap()
        } else {
            &mut self.root_component
        }
    }

    fn parsing_group(&self) -> bool {
        !self.active_groups.is_empty()
    }

    fn active_group(&self) -> &FixGroup {
        self.active_groups.last().unwrap()
    }

    fn active_group_mut(&mut self) -> &mut FixGroup {
        self.active_groups.last_mut().unwrap()
    }

    fn tag_in_group(&mut self, tag: i32) -> bool {
        // from cheapest to more expensive check
        !self.last_iteration() || self.known_group_tag(tag) || self.pending_tag_in_last_instance()
    }

    fn pending_tag_in_last_instance(&mut self) -> bool {
        let mut clean: Vec<i32> = Vec::new();
        for known_tag in self.active_group().known_tags.iter() {
            if let Some(tag_index) = self.get_next_index_of_tag(*known_tag) {
                if self.index_belongs_to_current_group(*tag_index) {
                    break;
                }
            }
            clean.push(*known_tag);  // optimization
        }

        for to_clean in clean {
            self.active_group_mut().known_tags.remove(&to_clean);
        }

        !self.active_group().known_tags.is_empty()
    }

    fn index_belongs_to_current_group(&self, tag_index: usize) -> bool {
        if let Some(delimiter_index) = self.get_next_index_of_tag(self.active_group().delimiter) {
            return tag_index < *delimiter_index
        }
        true
    }

    fn get_next_index_of_tag(&self, tag: i32) -> Option<&usize> {
        self.pending_tag_indices.get(&tag).unwrap().front()
    }

    fn known_group_tag(&self, tag: i32) -> bool {
        self.active_group().known_tags.contains(&tag)
    }

    fn last_iteration(&self) -> bool {
        self.active_group().current_iteration == self.active_group().repetitions
    }
}
