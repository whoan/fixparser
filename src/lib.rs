use serde::{
    Serialize, Serializer, ser::{SerializeMap}
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct FixComponent(Vec<FixEntity>);

#[derive(Debug, Clone)]
enum FixEntity {
    Field(i32, String),
    Group(i32, Vec<FixComponent>),
}

impl Serialize for FixComponent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for entity in &self.0 {
            match entity {
                FixEntity::Field(ref a, ref b) => {
                    map.serialize_entry(a, b)?;
                }
                FixEntity::Group(ref a, ref b) => {
                    map.serialize_entry(a, b)?;
                },
            }
        }
        map.end()
    }
}

impl FixEntity {
    fn get_field_value<'a>(fix_entity: &'a FixEntity) -> &'a str {
        if let FixEntity::Field(_dummy, repetitions) = fix_entity {
            println!("Repetitions {} - {}", _dummy, repetitions);
            return repetitions
        }
        panic!("ill-formated FIX");
    }

    fn get_field_key(fix_entity: &FixEntity) -> i32 {
        match fix_entity {
            FixEntity::Field(key, _dummy) => return *key,
            FixEntity::Group(key, _dummy) => return *key,
        }
    }
}

// is that useful?
fn get<T: std::str::FromStr>(input: &str) -> T where <T as std::str::FromStr>::Err: std::fmt::Debug  {
    input.parse().unwrap()
}

struct ActiveGroup {
    delimiter: i32,
    known_tags: HashSet<i32>,
    repetitions: i32,
    current_iteration: i32,
    group: FixEntity,
    candidate_indices: HashMap<i32, usize>, // store indices of candidate in-group tags
}

impl ActiveGroup {
    pub fn new(delimiter: i32, index_first_delimiter: usize, component: &mut FixComponent) -> Self {
        let group_instance = FixComponent(component.0.drain(index_first_delimiter..).collect());
        let known_tags = group_instance.0.iter().map(|fix_entity| FixEntity::get_field_key(fix_entity)).collect();
        let repetitions: i32 = get(FixEntity::get_field_value(&component.0.last().unwrap()));
        // bad variable name, as in FIX
        let no_tag = FixEntity::get_field_key(&component.0.last().unwrap());
        component.0.pop();
        Self {
            delimiter,
            repetitions,
            known_tags,
            current_iteration: 1,
            group: FixEntity::Group(no_tag, vec![group_instance]),
            candidate_indices: HashMap::new(),
        }
    }
}

pub struct FixMessage {
    candidate_indices: HashMap<i32, usize>, // store indices of candidate in-group tags
    pub root_component: FixComponent,
    active_groups: Vec<ActiveGroup>, // in case an instance but the first, have a nested repeating group. not sure if it can happen
    // A, B, no_C=3, C1, C2, C1, no_D=2, D1, D2, D1, D2, C2, C1, C2
    //        ^                    ^
    //    start group C       start group D (in second instance of group C)
}

impl FixMessage {
    fn new() -> Self {
        Self {
            candidate_indices: HashMap::new(),
            root_component: FixComponent(Vec::new()),
            active_groups: Vec::new(),
        }
    }

    pub fn from_raw(fix_message: &str) -> FixMessage {
        // TODO: trim in advance // should start with 8=
        let fix_field_separator: char = 0x01_u8.into();
        let mut message = FixMessage::new();
        for tag_value in fix_message.split(fix_field_separator) {
            let tag_value: Vec<&str> = tag_value.split('=').collect();
            if tag_value.len() > 1 {
                message.add_tag_value(
                    tag_value[0].parse().unwrap_or(0),
                    String::from(tag_value[1]),
                );
            }
        }
        message
    }

    fn merge_fields_into_group(&mut self, tag: i32) {
        println!("");
        println!("INFO: Group detected");

        let index_first_delimiter = self.get_index_first_delimiter(tag);
        //println!("Index first delimiter {}", index_first_delimiter);
        if self.parsing_group() {
            let group = match self.current_group_instance() {
                FixEntity::Group(ref _dummy, group) => ActiveGroup::new(tag, index_first_delimiter, &mut group.last_mut().unwrap()),
                _ => panic!("a group was expected")
            };
            self.active_groups.push(group);
        } else {
            self.active_groups.push(ActiveGroup::new(tag, index_first_delimiter, &mut self.root_component));
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

    fn add_tag_value(&mut self, tag: i32, value: String) {
        println!("Adding {} - {}", tag, value);
        if self.repeated_tag(tag) {
            // group detected
            self.merge_fields_into_group(tag);
        }

        if self.parsing_group() && !self.tag_in_group(tag) {
            println!("");
            println!("INFO: Stop parsing group");
            let group = self.active_groups.pop().unwrap().group;
            self.get_parent().0.push(group);
        }

        if self.parsing_group() && self.tag_in_group(tag) {
            self.active_group_mut().known_tags.insert(tag);
            let new_iteration = tag == self.active_group().delimiter;
            //println!("INFO: In group tag {} - delimiter {}", tag, self.active_group().delimiter);
            if new_iteration {
                self.active_group_mut().current_iteration += 1;
            }
            let mut index = 0;
            if let FixEntity::Group(ref _dummy, ref mut group) = &mut self.current_group_instance() {
                if new_iteration {
                    group.push(FixComponent(Vec::new()));
                }
                group.last_mut().unwrap().0.push(FixEntity::Field(tag, value));
                index = group.last_mut().unwrap().0.len()-1;
            }
            if new_iteration {
                self.clear_candidates();
            } else {
                self.register_candidate(tag, index);
            }
            return;
        }
        self.root_component.0.push(FixEntity::Field(tag, value));
        self.register_candidate(tag, self.root_component.0.len()-1);
    }

    fn current_group_instance(&mut self) -> &mut FixEntity {
        &mut self.active_group_mut().group
    }

    fn get_parent(&mut self) -> &mut FixComponent {
        if self.parsing_group() {
            if let FixEntity::Group(ref _dummy, ref mut group) = &mut self.active_group_mut().group {
                group.last_mut().unwrap()
            } else {
                panic!("should be inside a group");
            }
        } else {
            &mut self.root_component
        }
    }

    fn parsing_group(&self) -> bool {
        self.active_groups.len() > 0
    }

    fn active_group(&self) -> &ActiveGroup {
        self.active_groups.last().unwrap()
    }

    fn active_group_mut(&mut self) -> &mut ActiveGroup {
        self.active_groups.last_mut().unwrap()
    }

    fn tag_in_group(&self, tag: i32) -> bool {
        // TODO: check some more fields ahead as an improvemet -> not trivial
        // I could alternatively create "corrections", which appends fields to the last group if it was not finished
        !self.last_itertion() || self.known_group_tag(tag)
    }

    fn known_group_tag(&self, tag: i32) -> bool {
        self.active_group().known_tags.contains(&tag)
    }

    fn last_itertion(&self) -> bool {
        self.active_group().current_iteration == self.active_group().repetitions
    }
}
