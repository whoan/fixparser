use serde::{ser::SerializeMap, Serialize, Serializer};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
enum FixEntity {
    Field(i32, String),
    Group(i32, Vec<FixComponent>),
}

impl FixEntity {
    fn get_field_value(fix_entity: &FixEntity) -> &str {
        if let FixEntity::Field(_dummy, repetitions) = fix_entity {
            println!(". Repetitions {} - {}", _dummy, repetitions);
            println!();
            return repetitions;
        }
        panic!("ill-formated FIX");
    }

    fn get_field_key(fix_entity: &FixEntity) -> i32 {
        match fix_entity {
            FixEntity::Field(key, _dummy) => *key,
            FixEntity::Group(key, _dummy) => *key,
        }
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
                FixEntity::Field(ref a, ref b) => {
                    map.serialize_entry(a, b)?;
                }
                FixEntity::Group(ref a, ref b) => {
                    map.serialize_entry(a, b)?;
                }
            }
        }
        map.end()
    }
}

// is that useful?
fn get<T: std::str::FromStr>(input: &str) -> T
where
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    input.parse().unwrap()
}

struct ActiveGroup {
    delimiter: i32,
    repetitions: i32,
    current_iteration: i32,
    group: FixEntity,
    known_tags: HashSet<i32>, // tags we know that belong to this group
    candidate_indices: HashMap<i32, usize>, // store pending_indices of candidate nested group
}

impl ActiveGroup {
    pub fn new(delimiter: i32, index_first_delimiter: usize, component: &mut FixComponent) -> Self {
        let group_instance =
            FixComponent::new(component.entities.drain(index_first_delimiter..).collect());
        let known_tags = group_instance
            .entities
            .iter()
            .map(|fix_entity| FixEntity::get_field_key(fix_entity))
            .collect();
        let repetitions: i32 = get(FixEntity::get_field_value(
            &component.entities.last().unwrap()
        ));
        // bad variable name, as in FIX
        let no_tag = FixEntity::get_field_key(&component.entities.last().unwrap());
        component.entities.pop();
        Self {
            delimiter,
            repetitions,
            current_iteration: 1,
            group: FixEntity::Group(no_tag, vec![group_instance]),
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
    pending_indices: HashMap<i32, VecDeque<usize>>,
    candidate_indices: HashMap<i32, usize>,
    pub root_component: FixComponent,
    active_groups: Vec<ActiveGroup>, // in case an instance but the first, have a nested repeating group.
    current_index: usize, // for debugging
    // A, B, no_C=3, C1, C2, C1, no_D=2, D1, D2, D1, D2, C2, C1, C2
    //        ^                   ^
    //   start group C       start group D (in second instance of group C)
}

impl FixMessage {
    fn new() -> Self {
        Self {
            pending_indices: HashMap::new(),
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
                    .pending_indices
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
            message.add_tag_value(tag_value.0, String::from(tag_value.1));
            message.current_index += 1;
            end_of_message_found = tag_value.0 == 10;
        }

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

    fn open_group(&mut self, tag: i32) {
        print!("{}INFO: Group detected", self.get_spaces());
        // merge fields into group
        let index_first_delimiter = self.get_index_first_delimiter(tag);
        //println!("Index first delimiter {}", index_first_delimiter);
        if self.parsing_group() {
            let group = match self.current_group_instance() {
                FixEntity::Group(ref _dummy, group) => {
                    ActiveGroup::new(tag, index_first_delimiter, &mut group.last_mut().unwrap())
                }
                _ => panic!("a group was expected"),
            };
            self.active_groups.push(group);
        } else {
            self.active_groups.push(ActiveGroup::new(
                tag,
                index_first_delimiter,
                &mut self.root_component,
            ));
        }
    }

    fn get_candidates(&mut self) -> &mut HashMap<i32, usize> {
        if self.parsing_group() {
            &mut self.active_group_mut().candidate_indices
        } else {
            &mut self.candidate_indices
        }
    }

    fn get_index_first_delimiter(&mut self, tag: i32) -> usize {
        *self.get_candidates().get(&tag).unwrap()
    }

    fn register_candidate(&mut self, tag: i32, index: usize) {
        self.get_candidates().insert(tag, index);
    }

    fn clear_candidates(&mut self) {
        println!("Clearing");
        self.get_candidates().clear();
    }

    fn repeated_tag(&mut self, tag: i32) -> bool {
        self.get_candidates().contains_key(&tag)
    }

    fn remove_pending_tag(&mut self, tag: i32) {
        self.pending_indices.get_mut(&tag).unwrap().pop_front();
    }

    fn close_group(&mut self) {
        println!("{}INFO: Stop parsing group", self.get_spaces());
        println!();
        let group = self.active_groups.pop().unwrap().group;
        self.get_parent().entities.push(group);
    }

    fn add_tag_value(&mut self, tag: i32, value: String) {
        println!("{}Index {} - Adding {} - {}", self.get_spaces(), self.current_index, tag, value);

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
        //println!("INFO: In group tag {} - delimiter {}", tag, self.active_group().delimiter);
        if new_iteration {
            self.active_group_mut().current_iteration += 1;
        }
        let mut index = 0;
        if let FixEntity::Group(ref _dummy, ref mut group) = &mut self.current_group_instance()
        {
            if new_iteration {
                group.push(FixComponent::new(Vec::new()));
            }
            group
                .last_mut()
                .unwrap()
                .entities
                .push(FixEntity::Field(tag, value));
            index = group.last_mut().unwrap().entities.len() - 1;
        }
        if new_iteration {
            self.clear_candidates();
        } else {
            self.register_candidate(tag, index);
        }
    }

    fn get_spaces(&self) -> String {
        let mut spaces: Vec<char> = Vec::new();
        spaces.resize(self.active_groups.len() * 2, ' ');
        spaces.iter().collect()
    }

    fn current_group_instance(&mut self) -> &mut FixEntity {
        &mut self.active_group_mut().group
    }

    fn set_known_tag_in_group(&mut self, tag: i32) {
        self.active_group_mut().insert_known_tag(tag);
    }

    fn get_parent(&mut self) -> &mut FixComponent {
        if self.parsing_group() {
            if let FixEntity::Group(ref _dummy, ref mut group) = &mut self.active_group_mut().group
            {
                group.last_mut().unwrap()
            } else {
                panic!("should be inside a group");
            }
        } else {
            &mut self.root_component
        }
    }

    fn parsing_group(&self) -> bool {
        !self.active_groups.is_empty()
    }

    fn active_group(&self) -> &ActiveGroup {
        self.active_groups.last().unwrap()
    }

    fn active_group_mut(&mut self) -> &mut ActiveGroup {
        self.active_groups.last_mut().unwrap()
    }

    fn tag_in_group(&mut self, tag: i32) -> bool {
        !self.last_iteration() || self.known_group_tag(tag) || self.pending_tag_in_last_instance()
    }

    fn pending_tag_in_last_instance(&mut self) -> bool {
        let mut clean: Vec<i32> = Vec::new();
        for known_tag in self.active_group().known_tags.iter() {
            print!("{}known_tag {} - current_index {}", self.get_spaces(), known_tag, self.current_index);
            if let Some(known_tag_index) = self.pending_indices.get(known_tag).unwrap().back() {
                print!(" - known_tag_index {:?}", known_tag_index);
                if self.index_belongs_to_current_group(*known_tag_index) {
                    println!(" -> Pending");
                    break;
                }
            }
            println!(" -> Clean");
            clean.push(*known_tag);
        }
        // optimization (can I remove elements from known_tags somwehere else?)
        for to_clean in clean {
            self.active_group_mut().known_tags.remove(&to_clean);
        }
        !self.active_group().known_tags.is_empty()
    }

    fn index_belongs_to_current_group(&self, index: usize) -> bool {
        let &tag_indices = &self.pending_indices.get(&self.active_group().delimiter).unwrap();
        if let Some(delimiter_index) = tag_indices.back() {
            print!(" - delimiter ({}) index {}", self.active_group().delimiter, delimiter_index);
            return index < *delimiter_index
        }
        true
    }

    fn known_group_tag(&self, tag: i32) -> bool {
        self.active_group().known_tags.contains(&tag)
    }

    fn last_iteration(&self) -> bool {
        self.active_group().current_iteration == self.active_group().repetitions
    }
}
