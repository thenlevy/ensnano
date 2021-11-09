use iced::{
    button, scrollable, text_input, tooltip, Button, Column, Container, Element, Row, Scrollable,
    Space, TextInput, Tooltip,
};
pub use iced_aw::Icon;
use iced_native::keyboard::Modifiers;
use iced_wgpu::Text;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::convert::TryInto;

#[macro_use]
extern crate serde_derive;
extern crate serde;

mod drag_drop_target;
pub mod element;
mod hoverable_button;
pub mod theme;
mod tree;

pub use element::*;
use rand::{rngs::ThreadRng, Rng};
use theme::Theme;
pub use tree::{GroupId, OrganizerTree};

use drag_drop_target::*;

type HoverableButton<'a, Message> = hoverable_button::Button<'a, Message, iced_wgpu::Renderer>;

const LEVEL0_SPACING: u16 = 3;
const LEVELS_SPACING: u16 = 2;
const ICON_SIZE: u16 = 10;
const SECTION_ID: usize = usize::MAX;

#[derive(Clone, Debug)]
pub enum OrganizerMessage<E: OrganizerElement> {
    InternalMessage(InternalMessage<E>),
    Selection(Vec<E::Key>, Option<GroupId>),
    Candidates(Vec<E::Key>),
    ElementUpdate(Vec<BTreeMap<E::Key, E>>),
    NewAttribute(E::Attribute, Vec<E::Key>),
    NewTree(OrganizerTree<E::Key>),
    NewGroup {
        group_id: GroupId,
        elements_selected: Vec<E::Key>,
        new_tree: OrganizerTree<E::Key>,
    },
}

#[derive(Clone, Debug)]
pub struct InternalMessage<E: OrganizerElement>(OrganizerMessage_<E>);

type NodeId = Vec<usize>;

fn get_group_id(id: &[usize]) -> Option<&[usize]> {
    match id.get(0) {
        Some(n) if *n < SECTION_ID => Some(&id[..]),
        _ => None,
    }
}

fn get_section_id(id: &[usize]) -> Option<&[usize]> {
    match id.get(0) {
        Some(&SECTION_ID) => Some(&id[1..]),
        _ => None,
    }
}

fn get_element<'a, E: OrganizerElement>(
    sections: &'a [Section<E>],
    key: &'a E::Key,
) -> Option<&'a E> {
    let s_id: usize = key.section().into();
    sections.get(s_id).and_then(|s| s.content.get(key))
}

impl<E: OrganizerElement> OrganizerMessage<E> {
    fn expand(id: NodeId, expanded: bool) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::Expand { id, expanded }))
    }

    fn node_selected(id: NodeId) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::NodeSelected { id }))
    }

    fn node_hovered(id: NodeId, hovered_in: bool) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::NodeHovered {
            id,
            hovered_in,
        }))
    }

    fn key_hovered(key: E::Key, hovered_in: bool) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::KeyHovered {
            key,
            hovered_in,
        }))
    }

    fn eddit(id: NodeId) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::Eddit { id }))
    }

    fn delete(id: NodeId) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::Delete { id }))
    }

    fn name_input(name: String) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::NameInput { name }))
    }

    fn stop_eddit() -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::StopEddit))
    }

    fn element_selected(key: E::Key) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::ElementSelected { key }))
    }

    fn new_group() -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::NewGroup))
    }

    fn dragging(key: Identifier<E::Key>) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::Dragging(key)))
    }

    fn drag_dropped(key: Identifier<E::Key>) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::DragDropped(key)))
    }

    fn attribute_selected(attribute: E::Attribute, id: NodeId) -> Self {
        Self::InternalMessage(InternalMessage(OrganizerMessage_::AttributeSelected {
            attribute,
            id,
        }))
    }
}

#[derive(Clone, Debug)]
enum OrganizerMessage_<E: OrganizerElement> {
    Expand { id: NodeId, expanded: bool },
    NodeSelected { id: NodeId },
    NodeHovered { id: NodeId, hovered_in: bool },
    KeyHovered { key: E::Key, hovered_in: bool },
    ElementSelected { key: E::Key },
    Eddit { id: NodeId },
    StopEddit,
    NameInput { name: String },
    NewGroup,
    Delete { id: NodeId },
    DragDropped(Identifier<E::Key>),
    Dragging(Identifier<E::Key>),
    AttributeSelected { attribute: E::Attribute, id: NodeId },
}

pub struct Organizer<E: OrganizerElement> {
    rng_thread: ThreadRng,
    groups: Vec<GroupContent<E>>,
    sections: Vec<Section<E>>,
    scroll_state: scrollable::State,
    theme: Theme,
    width: iced::Length,
    edditing: Option<GroupId>,
    modifiers: Modifiers,
    selected_nodes: BTreeSet<NodeId>,
    dragging: BTreeSet<Identifier<E::Key>>,
    new_group_button: button::State,
    hovered_in: Option<NodeId>,
    last_read_tree: *const OrganizerTree<E::Key>,
    must_update_tree: bool,
    group_to_node: HashMap<GroupId, NodeId>,
}

impl<E: OrganizerElement> Organizer<E> {
    pub fn new() -> Self {
        let rng = rand::thread_rng();
        let mut sections = Vec::new();
        let mut i = 0usize;
        let mut section: Result<<E::Key as ElementKey>::Section, _> = i.try_into();
        while let Ok(s) = section {
            log::info!("section {:?}, {:?}", i, s);
            let new_section: Section<E> = Section::new(i, E::Key::name(s));
            sections.push(new_section);
            i += 1;
            section = i.try_into();
        }
        Self {
            rng_thread: rng,
            groups: vec![],
            sections,
            scroll_state: Default::default(),
            theme: Theme::grey(),
            width: iced::Length::Units(300),
            edditing: None,
            modifiers: Modifiers::default(),
            selected_nodes: BTreeSet::new(),
            dragging: BTreeSet::new(),
            new_group_button: Default::default(),
            hovered_in: None,
            last_read_tree: std::ptr::null(),
            must_update_tree: false,
            group_to_node: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.groups = vec![]
    }

    pub fn new_modifiers(&mut self, modifiers: Modifiers) {
        self.modifiers = modifiers;
    }

    pub fn set_width(&mut self, width: u16) {
        self.width = iced::Length::Units(width);
    }

    pub fn view(&mut self, selection: BTreeSet<E::Key>) -> Element<OrganizerMessage<E>> {
        self.hovered_in = None;
        let mut ret = Scrollable::new(&mut self.scroll_state)
            .width(self.width)
            .spacing(LEVEL0_SPACING);
        for c in self.groups.iter_mut() {
            ret = ret.push(
                Row::new().push(tabulation()).push(
                    c.view(
                        &self.theme,
                        &self.sections,
                        &selection,
                        &self.selected_nodes,
                    )
                    .width(iced::Length::FillPortion(8)),
                ),
            )
        }
        for s in self.sections.iter_mut() {
            ret = ret.push(
                Row::new().push(tabulation()).push(
                    s.view(&self.theme, &selection)
                        .width(iced::Length::FillPortion(8)),
                ),
            )
        }
        let mut new_group_button = Button::new(&mut self.new_group_button, Text::new("New Group"));
        if !selection.is_empty() {
            new_group_button = new_group_button.on_press(OrganizerMessage::new_group());
        }
        let new_group_tooltip = Tooltip::new(
            new_group_button,
            "Create new_group from selection",
            tooltip::Position::FollowCursor,
        );
        let title_row = Row::new().push(new_group_tooltip);
        let column = Column::new().push(title_row).push(ret);
        Container::new(column).style(self.theme.level(0)).into()
    }

    pub fn push_content(&mut self, content: Vec<E::Key>, group_name: String) -> GroupId {
        let id = vec![self.groups.len()];
        let new_group = GroupContent::new(content, group_name, id.clone(), &mut self.rng_thread);
        let ret = new_group
            .get_group_id()
            .expect("new group should have an Id");
        self.groups.push(new_group);
        self.edditing = Some(ret);
        ret
    }

    pub fn message(
        &mut self,
        message: &InternalMessage<E>,
        selection: &BTreeSet<E::Key>,
    ) -> Option<OrganizerMessage<E>> {
        log::trace!("{:?}", message);
        match &message.0 {
            OrganizerMessage_::Expand { id, expanded } => self.expand(id, *expanded),
            OrganizerMessage_::NodeSelected { id } => {
                let add = self.modifiers.command() || self.modifiers.shift();
                let (new_selection, new_group) = self.select_node(id, add, selection.clone());
                return Some(OrganizerMessage::Selection(
                    new_selection.into_iter().collect(),
                    new_group,
                ));
            }
            OrganizerMessage_::Eddit { id } => {
                if let Some(group_id) = self.get_group(id).and_then(|g| g.get_group_id()) {
                    self.start_edditing(group_id)
                } else {
                    log::error!("Could not get group id");
                }
            }
            OrganizerMessage_::NameInput { name } => self.eddit_name(name.clone()),
            OrganizerMessage_::StopEddit => {
                self.stop_edditing();
                return Some(OrganizerMessage::NewTree(self.tree()));
            }
            OrganizerMessage_::ElementSelected { key } => {
                let new_selection = if self.modifiers.command() || self.modifiers.shift() {
                    let mut new_selection = selection.clone();
                    Self::add_selection(&mut new_selection, key, true);
                    new_selection
                } else {
                    self.selected_nodes = BTreeSet::new();
                    self.set_selection(key, selection.clone())
                };
                return Some(OrganizerMessage::Selection(
                    new_selection.into_iter().collect(),
                    None,
                ));
            }
            OrganizerMessage_::NewGroup => {
                let new_group_id = self.push_content(
                    selection.iter().cloned().collect(),
                    String::from("New group"),
                );
                return Some(OrganizerMessage::NewGroup {
                    new_tree: self.tree(),
                    group_id: new_group_id,
                    elements_selected: selection.iter().cloned().collect(),
                });
            }
            OrganizerMessage_::Delete { id } => {
                self.stop_edditing();
                self.pop_id(id);
                return Some(OrganizerMessage::NewTree(self.tree()));
            }
            OrganizerMessage_::Dragging(k) => {
                self.dragging.clear();
                self.dragging.insert(k.clone());
            }
            OrganizerMessage_::DragDropped(k) => self.drag_drop(k),
            OrganizerMessage_::NodeHovered { id, hovered_in } => {
                return self.hover(id, *hovered_in)
            }
            OrganizerMessage_::KeyHovered { key, hovered_in } => {
                return self.key_hover(key.clone(), *hovered_in)
            }
            OrganizerMessage_::AttributeSelected { attribute, id } => {
                let keys = self.get_keys_below(id);
                return Some(OrganizerMessage::NewAttribute(attribute.clone(), keys));
            }
        }
        None
    }

    fn hover(&mut self, id: &NodeId, hovered_in: bool) -> Option<OrganizerMessage<E>> {
        if hovered_in {
            self.get_group(id)
                .map(|g| OrganizerMessage::Candidates(g.get_all_elements_below()))
                .or(self
                    .get_section_id(id)
                    .map(|s| OrganizerMessage::Candidates(s.get_all_keys())))
        } else if self.hovered_in.is_none() {
            Some(OrganizerMessage::Candidates(vec![]))
        } else {
            None
        }
    }

    fn key_hover(&mut self, key: E::Key, hovered_in: bool) -> Option<OrganizerMessage<E>> {
        if hovered_in {
            Some(OrganizerMessage::Candidates(vec![key]))
        } else if self.hovered_in.is_none() {
            Some(OrganizerMessage::Candidates(vec![]))
        } else {
            None
        }
    }

    pub fn notify_selection(&mut self, selected_group: Option<GroupId>) {
        log::info!("Notified of selection");
        let selected_node = selected_group.and_then(|g_id| self.group_to_node.get(&g_id).cloned());
        self.selected_nodes = BTreeSet::new();
        if let Some(node_id) = selected_node {
            self.selected_nodes.insert(node_id);
        }
    }

    fn add_selection(selection: &mut BTreeSet<E::Key>, key: &E::Key, may_remove: bool) {
        if selection.contains(key) {
            if may_remove {
                selection.remove(key);
            }
        } else {
            selection.insert(key.clone());
        }
    }

    fn select_node(
        &mut self,
        id: &NodeId,
        add: bool,
        mut current_selection: BTreeSet<E::Key>,
    ) -> (BTreeSet<E::Key>, Option<GroupId>) {
        let group_id = if add {
            if self.selected_nodes.contains(id) {
                let keys: BTreeSet<E::Key> = self.get_keys_below(id).into_iter().collect();
                for key in keys.iter() {
                    current_selection.remove(key);
                }
                self.selected_nodes.remove(id);
            } else {
                let keys: BTreeSet<E::Key> = self.get_keys_below(id).into_iter().collect();
                for key in keys.iter() {
                    Self::add_selection(&mut current_selection, key, false);
                }
                self.selected_nodes.insert(id.clone());
            };
            None
        } else {
            if self.selected_nodes.len() == 1 && self.selected_nodes.contains(id) {
                self.selected_nodes = BTreeSet::new();
                current_selection = BTreeSet::new();
                None
            } else {
                self.selected_nodes = BTreeSet::new();
                self.selected_nodes.insert(id.clone());
                current_selection = self.get_keys_below(id).iter().cloned().collect();
                self.get_group(&id).and_then(|g| g.get_group_id())
            }
        };
        log::info!("Selected nodes = {:?}", self.selected_nodes);
        (current_selection, group_id)
    }

    fn get_keys_below(&self, id: &NodeId) -> Vec<E::Key> {
        if let Some(group) = self.get_group(id) {
            group.get_all_elements_below()
        } else if let Some(section) = self.get_section_id(id) {
            section.get_all_keys()
        } else {
            vec![]
        }
    }

    fn get_section_id<'a, 'b>(&'a self, id: &'b NodeId) -> Option<&'a Section<E>> {
        if let Some(section_id) = get_section_id(id) {
            let s_id = section_id.get(0)?;
            self.sections.get(*s_id)
        } else {
            None
        }
    }

    fn get_group<'a, 'b>(&'a self, id: &'b NodeId) -> Option<&'a GroupContent<E>> {
        if let Some(group_id) = get_group_id(id) {
            if id.len() < 2 {
                self.groups.get(group_id[0])
            } else {
                self.groups
                    .get(group_id[0])
                    .and_then(|g| g.get_group(&id[1..]))
            }
        } else {
            None
        }
    }

    fn set_selection(
        &mut self,
        key: &E::Key,
        mut current_selection: BTreeSet<E::Key>,
    ) -> BTreeSet<E::Key> {
        if current_selection.len() == 1 && current_selection.contains(key) {
            current_selection.clear();
        } else {
            current_selection.clear();
            current_selection.insert(key.clone());
        };
        current_selection
    }

    fn start_edditing(&mut self, id: GroupId) {
        self.stop_edditing();
        if let Some(id_slice) = self
            .group_to_node
            .get(&id)
            .and_then(|node_id| get_group_id(node_id))
        {
            log::info!("start edditing {:?}", id);
            self.groups[id_slice[0]].start_edditing(&id_slice[1..]);
            self.edditing = Some(id);
        }
    }

    pub fn has_keyboard_priority(&self) -> bool {
        self.edditing.is_some()
    }

    fn stop_edditing(&mut self) {
        let node_id = self
            .edditing
            .as_ref()
            .and_then(|g_id| self.group_to_node.get(g_id).clone())
            .cloned();
        if let Some(id) = node_id.as_ref().and_then(|v| get_group_id(v)) {
            self.groups[id[0]].stop_edditing(&id[1..]);
        }
        self.edditing = None;
    }

    fn eddit_name(&mut self, name: String) {
        let node_id = self
            .edditing
            .as_ref()
            .and_then(|g_id| self.group_to_node.get(g_id).clone())
            .cloned();
        if let Some(id) = node_id.as_ref().and_then(|v| get_group_id(v)) {
            self.groups[id[0]].eddit_name(&id[1..], name);
        } else {
            println!("ERROR receive name input but self.edditing is None");
        }
    }

    fn expand(&mut self, id: &[usize], expanded: bool) {
        if let Some(id) = get_group_id(id) {
            self.groups[id[0]].expand(&id[1..], expanded)
        } else if let Some(id) = get_section_id(id) {
            self.sections[id[0]].expand(expanded)
        }
    }

    fn recompute_id(&mut self) {
        self.groups.retain(|c| !c.is_placeholder());
        self.group_to_node.clear();
        for (i, c) in self.groups.iter_mut().enumerate() {
            c.recompute_id(vec![i], &mut self.group_to_node)
        }
    }

    pub fn tree(&self) -> OrganizerTree<E::Key> {
        let groups = self.groups.iter().filter_map(|g| g.tree()).collect();
        OrganizerTree::Node {
            name: "root".to_owned(),
            childrens: groups,
            expanded: true,
            id: None,
        }
    }

    #[must_use = "If the tree has been updated, the program must be notified"]
    pub fn read_tree(&mut self, tree: &OrganizerTree<E::Key>) -> bool {
        if self.last_read_tree != tree {
            self.last_read_tree = tree;
            if let OrganizerTree::Node { childrens, .. } = tree {
                self.groups = childrens
                    .iter()
                    .map(|g| {
                        GroupContent::read_tree(g, &mut self.rng_thread, &mut self.must_update_tree)
                    })
                    .collect();
            } else {
                self.groups = vec![];
            }
            self.recompute_id();
            self.update_attributes();
            if let Some(group_id) = self.edditing {
                self.start_edditing(group_id)
            }
        }
        let ret = self.must_update_tree;
        self.must_update_tree = false;
        ret
    }

    fn pop_id(&mut self, id: &NodeId) -> Option<GroupContent<E>> {
        if let Some(id) = get_group_id(id) {
            let ret;
            if id.len() < 2 {
                if self.groups.len() > id[0] {
                    ret = Some(self.groups.remove(id[0]))
                } else {
                    ret = None;
                }
            } else {
                ret = self.groups.get_mut(id[0]).and_then(|c| c.pop_id(&id[1..]));
            }
            if ret.is_some() {
                self.recompute_id()
            }
            ret
        } else {
            None
        }
    }

    fn pop_id_no_recompute(&mut self, id: &NodeId) -> Option<GroupContent<E>> {
        if let Some(id) = get_group_id(id) {
            let ret;
            if id.len() < 2 {
                if self.groups.len() > id[0] {
                    ret = Some(std::mem::replace(
                        self.groups.get_mut(id[0]).unwrap(),
                        GroupContent::Placeholder,
                    ))
                } else {
                    ret = None;
                }
            } else {
                ret = self.groups.get_mut(id[0]).and_then(|c| c.pop_id(&id[1..]));
            }
            ret
        } else {
            None
        }
    }

    fn replace_id(&mut self, content: GroupContent<E>, id: &NodeId) {
        if let Some(id) = get_group_id(id) {
            if id.len() < 2 {
                if self.groups.len() < id[0] {
                    self.groups.push(content)
                } else {
                    self.groups.insert(id[0], content);
                }
            } else {
                // We unwrap because getting None would be the symptom of a serious bug
                self.groups
                    .get_mut(id[0])
                    .unwrap()
                    .replace_id(&id[1..], content)
            }
        }
    }

    fn add_at_id(&mut self, content: GroupContent<E>, id: &NodeId, from_top: bool) {
        if let Some(id) = get_group_id(id) {
            if id.len() < 2 {
                let insertion_point = if from_top { id[0] + 1 } else { id[0] };
                self.groups.insert(insertion_point, content);
            } else {
                self.groups
                    .get_mut(id[0])
                    .unwrap()
                    .add_at_id(&id[1..], content, from_top)
            }
        }
    }

    fn drag_drop(&mut self, k: &Identifier<E::Key>) {
        match k {
            Identifier::Group { id: id_dest } => {
                if let Some(identifer) = self.dragging.iter().next().cloned() {
                    match identifer {
                        id if id == k.clone() => (),
                        Identifier::Group { id } => self.move_id(&id, id_dest),
                        Identifier::Section { key } => self.add_key_at(key, id_dest),
                    }
                }
            }
            _ => (),
        }
        self.dragging = BTreeSet::new();
    }

    pub fn merge_ids(&mut self, id0: &NodeId, id1: &NodeId) {
        //TODO remove public once this is integrated in GUI
        if let Some(c1) = self.pop_id_no_recompute(id0) {
            if let Some(c2) = self.pop_id_no_recompute(id1) {
                let new_group_id = self.rng_thread.gen();
                let content = GroupContent::Node {
                    id: vec![],
                    name: String::from("new group"),
                    expanded: false,
                    childrens: vec![c2, c1],
                    view: NodeView::new(),
                    attributes: vec![None; E::all_repr().len()],
                    elements_below: BTreeSet::new(),
                    group_id: new_group_id,
                };
                self.replace_id(content, id1);
            } else {
                self.replace_id(c1, id0)
            }
            self.recompute_id()
        }
    }

    fn move_id(&mut self, source: &NodeId, dest: &NodeId) {
        if get_section_id(dest).is_some() {
            return;
        }
        if source.len() < dest.len() && dest[..source.len()] == source[..] {
            println!("prefix");
            return;
        }
        let from_top = source <= dest;
        if let Some(content) = self.pop_id_no_recompute(source) {
            self.add_at_id(content, dest, from_top);
            self.recompute_id()
        }
        self.must_update_tree = true;
    }

    fn add_key_at(&mut self, key: E::Key, dest: &NodeId) {
        if get_section_id(dest).is_some() {
            return;
        }
        if dest.len() < 2 {
            println!("I have not decided what to do when moving a key at the root level of the organizer");
        } else {
            if let Some(group) = self.groups.get_mut(dest[0]) {
                group.add_key_at(key, &dest[1..])
            }
        }
        self.recompute_id();
        self.must_update_tree = true;
    }

    pub fn update_elements(&mut self, elements: &[E]) {
        for s in self.sections.iter_mut() {
            s.elements.clear();
            s.content.clear();
        }
        for e in elements.iter() {
            let key = e.key();
            let section_id: usize = key.section().into();
            self.sections[section_id].add_element(e.clone());
        }
        self.update_attributes();
    }

    fn update_attributes(&mut self) {
        for g in self.groups.iter_mut() {
            g.update_attributes(&self.sections);
        }
        for s in self.sections.iter_mut() {
            s.update_attributes()
        }
    }
}

struct Section<E: OrganizerElement> {
    content: BTreeMap<E::Key, E>,
    id: NodeId,
    name: String,
    expanded: bool,
    view: NodeView<E>,
    elements: BTreeMap<E::Key, ElementView<E>>,
}

impl<E: OrganizerElement> Section<E> {
    fn new(id: usize, name: String) -> Self {
        let id = vec![SECTION_ID, id];
        Self {
            content: BTreeMap::new(),
            id,
            name,
            expanded: false,
            view: NodeView::new_section(),
            elements: BTreeMap::new(),
        }
    }

    fn expand(&mut self, expanded: bool) {
        self.expanded = expanded
    }

    fn view(
        &mut self,
        theme: &Theme,
        selection: &BTreeSet<E::Key>,
    ) -> Container<OrganizerMessage<E>> {
        let title_row = self
            .view
            .view(theme, &self.name, self.id.clone(), self.expanded, false);
        let mut ret = Column::new()
            .spacing(LEVELS_SPACING)
            .push(Element::new(title_row));
        if self.expanded {
            for (e_id, e) in self.elements.iter_mut() {
                ret = ret.push(
                    Row::new().push(tabulation()).push(
                        Container::new(Element::new(e.view(
                            theme,
                            &self.content[e_id],
                            selection,
                            None,
                        )))
                        .style(theme.level(1))
                        .width(iced::Length::FillPortion(8)),
                    ),
                )
            }
        }
        Container::new(ret).style(theme.level(0))
    }

    fn add_element(&mut self, element: E) {
        let key = element.key();
        self.content.insert(key.clone(), element);
        self.elements.insert(key, ElementView::new());
    }

    fn update_attributes(&mut self) {
        for (k, e) in self.content.iter() {
            if let Some(view) = self.elements.get_mut(k) {
                view.update_element(e)
            }
        }
    }

    fn get_all_keys(&self) -> Vec<E::Key> {
        self.content.keys().cloned().collect()
    }
}

/// A data structure whose view displays information about an element.
struct ElementView<E: OrganizerElement + 'static> {
    attribute_displayers: Vec<AttributeDisplayer<E::Attribute>>,
    button_state: hoverable_button::State,
    delete_button_state: button::State,
}

impl<E: OrganizerElement> ElementView<E> {
    fn new() -> Self {
        Self {
            attribute_displayers: vec![AttributeDisplayer::new(); E::all_repr().len()],
            button_state: Default::default(),
            delete_button_state: Default::default(),
        }
    }
    fn view(
        &mut self,
        theme: &Theme,
        element: &E,
        selection: &BTreeSet<E::Key>,
        deletable: Option<NodeId>,
    ) -> DragDropTarget<OrganizerMessage<E>, E::Key> {
        let selected = selection.contains(&element.key());
        let mut content = Row::new()
            .push(Text::new(element.display_name()))
            .push(Space::with_width(iced::Length::Fill));
        let identifier = match deletable.as_ref() {
            Some(id) => Identifier::Group { id: id.clone() },
            None => Identifier::Section {
                key: element.key().clone(),
            },
        };
        for ad in self.attribute_displayers.iter_mut() {
            if let Some(view) = ad.view() {
                let mut elt = BTreeSet::new();
                elt.insert(element.key());
                let elt_key = element.key();
                content = content.push(
                    view.map(move |m| OrganizerMessage::NewAttribute(m, vec![elt_key.clone()])),
                )
            }
        }
        if let Some(id) = deletable.clone() {
            content = content.push(
                Button::new(&mut self.delete_button_state, icon(Icon::Trash.into()))
                    .on_press(OrganizerMessage::delete(id)),
            );
        }
        let mut button = HoverableButton::new(&mut self.button_state, content)
            .on_press(OrganizerMessage::element_selected(element.key().clone()))
            .width(iced::Length::Fill)
            .style(theme.selected(selected));
        if let Some(id) = deletable {
            button = button
                .on_hovered_in(OrganizerMessage::node_hovered(id.clone(), true))
                .on_hovered_out(OrganizerMessage::node_hovered(id, false))
        } else {
            button = button
                .on_hovered_in(OrganizerMessage::key_hovered(element.key(), true))
                .on_hovered_out(OrganizerMessage::key_hovered(element.key(), false))
        }
        DragDropTarget::new(button, identifier).width(iced::Length::Fill)
    }

    fn update_attributes(&mut self, attributes: &[Option<E::Attribute>]) {
        for (i, a) in attributes.iter().enumerate() {
            self.attribute_displayers[i].update_attribute(a.clone())
        }
    }

    fn update_element(&mut self, element: &E) {
        for e in element.attributes() {
            self.attribute_displayers[e.repr().into()].update_attribute(Some(e.clone()))
        }
    }
}

/// A data structure whose view is a "title bar" for a group or a section
struct NodeView<E: OrganizerElement> {
    expansion_btn_state: button::State,
    title_button_state: hoverable_button::State,
    state: GroupState,
    attribute_displayers: Vec<AttributeDisplayer<E::Attribute>>,
}

impl<E: OrganizerElement> NodeView<E> {
    fn new() -> Self {
        Self {
            expansion_btn_state: Default::default(),
            title_button_state: Default::default(),
            state: GroupState::Iddle {
                eddit_button: Default::default(),
                delete_button: Default::default(),
            },
            attribute_displayers: vec![AttributeDisplayer::new(); E::all_repr().len()],
        }
    }

    fn new_section() -> Self {
        Self {
            expansion_btn_state: Default::default(),
            title_button_state: Default::default(),
            state: GroupState::NotEdditable,
            attribute_displayers: vec![],
        }
    }

    fn start_edditing(&mut self) {
        log::info!("reached view");
        self.state = GroupState::Edditing {
            input: text_input::State::focused(),
            delete_button: Default::default(),
            eddit_button: Default::default(),
        };
        if let GroupState::Edditing { input, .. } = &mut self.state {
            input.select_all()
        }
    }

    fn stop_edditing(&mut self) {
        self.state = GroupState::Iddle {
            eddit_button: Default::default(),
            delete_button: Default::default(),
        };
    }

    fn view(
        &mut self,
        theme: &Theme,
        name: &String,
        id: NodeId,
        expanded: bool,
        selected: bool,
    ) -> DragDropTarget<OrganizerMessage<E>, E::Key> {
        let level = id.len();
        let title_row = match &mut self.state {
            GroupState::Iddle {
                eddit_button,
                delete_button,
            } => {
                let mut row = Row::new();
                row = row.push(
                    Button::new(&mut self.expansion_btn_state, expand_icon(expanded))
                        .on_press(OrganizerMessage::expand(id.clone(), !expanded)),
                );
                row = row
                    .push(Text::new(name.clone()))
                    .push(Space::with_width(iced::Length::Fill));

                row = row.push(
                    Button::new(eddit_button, eddit_icon())
                        .on_press(OrganizerMessage::eddit(id.clone())),
                );

                for ad in self.attribute_displayers.iter_mut() {
                    if let Some(view) = ad.view() {
                        let id = id.clone();
                        row = row.push(
                            view.map(move |m| OrganizerMessage::attribute_selected(m, id.clone())),
                        )
                    }
                }

                row = row.push(
                    Button::new(delete_button, icon(Icon::Trash.into()))
                        .on_press(OrganizerMessage::delete(id.clone())),
                );
                row
            }
            GroupState::Edditing {
                input,
                delete_button,
                eddit_button,
            } => {
                let name = name.clone();
                let mut row = Row::new()
                    .push(
                        Button::new(&mut self.expansion_btn_state, expand_icon(expanded))
                            .on_press(OrganizerMessage::expand(id.clone(), !expanded)),
                    )
                    .push(
                        TextInput::new(input, "New group name...", &name, |s| {
                            OrganizerMessage::name_input(s)
                        })
                        .on_submit(OrganizerMessage::stop_eddit()),
                    )
                    .push(Space::with_width(iced::Length::Fill));

                row = row.push(
                    Button::new(eddit_button, eddit_icon())
                        .on_press(OrganizerMessage::stop_eddit()),
                );
                for ad in self.attribute_displayers.iter_mut() {
                    if let Some(view) = ad.view() {
                        let id = id.clone();
                        row = row.push(
                            view.map(move |m| OrganizerMessage::attribute_selected(m, id.clone())),
                        )
                    }
                }
                row = row.push(
                    Button::new(delete_button, icon(Icon::Trash.into()))
                        .on_press(OrganizerMessage::delete(id.clone())),
                );
                row
            }
            GroupState::NotEdditable => {
                let mut row = Row::new();
                row = row.push(
                    Button::new(&mut self.expansion_btn_state, expand_icon(expanded))
                        .on_press(OrganizerMessage::expand(id.clone(), !expanded)),
                );
                row = row.push(Text::new(name.clone()));
                row
            }
        };
        let mut button = HoverableButton::new(&mut self.title_button_state, title_row)
            .on_press(OrganizerMessage::node_selected(id.clone()))
            .on_hovered_in(OrganizerMessage::node_hovered(id.clone(), true))
            .on_hovered_out(OrganizerMessage::node_hovered(id.clone(), false))
            .width(iced::Length::Fill);
        if selected {
            button = button.style(theme.level_selected(level));
        } else {
            button = button.style(theme.level(level));
        }
        DragDropTarget::new(button, Identifier::Group { id: id.clone() }).width(iced::Length::Fill)
    }

    fn update_attributes(&mut self, attributes: &[Option<E::Attribute>]) {
        for (i, a) in attributes.iter().enumerate() {
            self.attribute_displayers[i].update_attribute(a.clone())
        }
    }
}

enum GroupContent<E: OrganizerElement> {
    Leaf {
        id: NodeId,
        element: E::Key,
        view: ElementView<E>,
        attributes: Vec<Option<E::Attribute>>,
    },
    Node {
        id: NodeId,
        name: String,
        expanded: bool,
        view: NodeView<E>,
        childrens: Vec<GroupContent<E>>,
        attributes: Vec<Option<E::Attribute>>,
        elements_below: BTreeSet<E::Key>,
        group_id: GroupId,
    },
    Placeholder,
}

pub enum GroupState {
    Iddle {
        eddit_button: button::State,
        delete_button: button::State,
    },
    Edditing {
        input: text_input::State,
        delete_button: button::State,
        eddit_button: button::State,
    },
    NotEdditable,
}

impl<E: OrganizerElement> GroupContent<E> {
    fn view(
        &mut self,
        theme: &Theme,
        sections: &[Section<E>],
        selection: &BTreeSet<E::Key>,
        selected_nodes: &BTreeSet<NodeId>,
    ) -> Container<OrganizerMessage<E>> {
        let level;
        let colummn = match self {
            Self::Node {
                name,
                expanded,
                childrens,
                view,
                id,
                ..
            } => {
                level = id.len();
                let selected = selected_nodes.contains(id);
                let title_row = view.view(theme, name, id.clone(), *expanded, selected);
                let mut ret = Column::new()
                    .spacing(LEVELS_SPACING)
                    .push(Element::new(title_row));
                if *expanded {
                    for c in childrens.iter_mut() {
                        ret = ret.push(
                            Row::new().push(tabulation()).push(
                                c.view(theme, sections, selection, selected_nodes)
                                    .width(iced::Length::FillPortion(8)),
                            ),
                        )
                    }
                }
                ret
            }
            Self::Leaf {
                view, element, id, ..
            } => {
                level = id.len();
                if let Some(element) = get_element(sections, element) {
                    Column::new()
                        .spacing(LEVELS_SPACING)
                        .push(Element::new(view.view(
                            theme,
                            element,
                            selection,
                            Some(id.clone()),
                        )))
                } else {
                    println!("WARNING viewing leaf owning deleted element");
                    Column::new()
                }
            }
            Self::Placeholder => unreachable!("Viewing a placeholder"),
        };
        Container::new(colummn).style(theme.level(level))
    }

    fn leaf(key: E::Key, id: NodeId) -> Self {
        Self::Leaf {
            id,
            element: key,
            view: ElementView::new(),
            attributes: vec![None; E::all_repr().len()],
        }
    }

    fn read_tree(
        tree: &OrganizerTree<E::Key>,
        rng: &mut ThreadRng,
        must_update_tree: &mut bool,
    ) -> Self {
        match tree {
            OrganizerTree::Leaf(k) => Self::Leaf {
                id: vec![],
                element: k.clone(),
                view: ElementView::new(),
                attributes: vec![None; E::all_repr().len()],
            },
            OrganizerTree::Node {
                name,
                childrens: content,
                expanded,
                id,
            } => {
                let childrens = content
                    .iter()
                    .map(|c| Self::read_tree(c, rng, must_update_tree))
                    .collect();
                let group_id = id.clone().unwrap_or_else(|| {
                    // when we generate a new identifier, we must notify the program that the tree
                    // is different
                    *must_update_tree = true;
                    rng.gen()
                });
                Self::Node {
                    childrens,
                    id: vec![],
                    name: name.clone(),
                    expanded: *expanded,
                    view: NodeView::new(),
                    attributes: vec![None; E::all_repr().len()],
                    elements_below: BTreeSet::new(),
                    group_id,
                }
            }
        }
    }

    fn new(content: Vec<E::Key>, name: String, id: NodeId, rng: &mut ThreadRng) -> Self {
        let childrens = content
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                let mut id = id.clone();
                id.push(i);
                Self::Leaf {
                    id,
                    element: e.clone(),
                    view: ElementView::new(),
                    attributes: vec![None; E::all_repr().len()],
                }
            })
            .collect();
        let group_id = rng.gen();
        Self::Node {
            id,
            childrens,
            name,
            expanded: false,
            view: NodeView::new(),
            attributes: vec![None; E::all_repr().len()],
            elements_below: BTreeSet::new(),
            group_id,
        }
    }

    fn start_edditing(&mut self, id: &[usize]) {
        if id.len() > 0 {
            match self {
                Self::Leaf { .. } => {
                    println!("ERROR ACCESSING A LEAF WITHOUT EXHAUSTING ID");
                }
                Self::Node { childrens, .. } => childrens[id[0]].start_edditing(&id[1..]),
                Self::Placeholder => unreachable!("Expanding a Placeholder"),
            }
        } else {
            match self {
                Self::Leaf { .. } => {
                    println!("ERROR ACCESSING A LEAF WITHOUT EXHAUSTING ID");
                }
                Self::Node { view, .. } => view.start_edditing(),
                Self::Placeholder => unreachable!("Expanding a Placeholder"),
            }
        }
    }

    fn stop_edditing(&mut self, id: &[usize]) {
        if id.len() > 0 {
            match self {
                Self::Leaf { .. } => {
                    println!("ERROR ACCESSING A LEAF WITHOUT EXHAUSTING ID");
                }
                Self::Node { childrens, .. } => childrens[id[0]].stop_edditing(&id[1..]),
                Self::Placeholder => unreachable!("Expanding a Placeholder"),
            }
        } else {
            match self {
                Self::Leaf { .. } => {
                    println!("ERROR ACCESSING A LEAF WITHOUT EXHAUSTING ID");
                }
                Self::Node { view, .. } => {
                    view.stop_edditing();
                }
                Self::Placeholder => unreachable!("Expanding a Placeholder"),
            }
        }
    }

    fn eddit_name(&mut self, id: &[usize], name: String) {
        if id.len() > 0 {
            match self {
                Self::Leaf { .. } => {
                    println!("ERROR ACCESSING A LEAF WITHOUT EXHAUSTING ID");
                }
                Self::Node { childrens, .. } => childrens[id[0]].eddit_name(&id[1..], name),
                Self::Placeholder => unreachable!("Expanding a Placeholder"),
            }
        } else {
            match self {
                Self::Leaf { .. } => {
                    println!("ERROR ACCESSING A LEAF WITHOUT EXHAUSTING ID");
                }
                Self::Node {
                    name: node_name, ..
                } => {
                    *node_name = name;
                }
                Self::Placeholder => unreachable!("Expanding a Placeholder"),
            }
        }
    }

    fn expand(&mut self, id: &[usize], expanded: bool) {
        if id.len() > 0 {
            match self {
                Self::Leaf { .. } => {
                    println!("ERROR ACCESSING A LEAF WITHOUT EXHAUSTING ID");
                }
                Self::Node { childrens, .. } => childrens[id[0]].expand(&id[1..], expanded),
                Self::Placeholder => unreachable!("Expanding a Placeholder"),
            }
        } else {
            match self {
                Self::Leaf { .. } => {
                    println!("ERROR ACCESSING A LEAF WITHOUT EXHAUSTING ID");
                }
                Self::Node {
                    expanded: expanded_ref,
                    ..
                } => {
                    *expanded_ref = expanded;
                }
                Self::Placeholder => unreachable!("Expanding a Placeholder"),
            }
        }
    }

    fn is_placeholder(&self) -> bool {
        match self {
            Self::Placeholder => true,
            _ => false,
        }
    }

    fn recompute_id(&mut self, id: NodeId, map: &mut HashMap<GroupId, NodeId>) {
        match self {
            Self::Leaf { id: id_ref, .. } => *id_ref = id,
            Self::Node {
                id: id_ref,
                childrens,
                group_id,
                ..
            } => {
                childrens.retain(|c| !c.is_placeholder());
                for (i, c) in childrens.iter_mut().enumerate() {
                    let mut id_child = id.clone();
                    id_child.push(i);
                    c.recompute_id(id_child, map);
                }
                *id_ref = id.clone();
                map.insert(*group_id, id);
            }
            Self::Placeholder => unreachable!("Recomputing id of a placeholder"),
        }
    }

    fn pop_id(&mut self, id: &[usize]) -> Option<Self> {
        match self {
            Self::Node { childrens, .. } => {
                if id.len() > 1 {
                    childrens.get_mut(id[0]).and_then(|c| c.pop_id(&id[1..]))
                } else {
                    childrens
                        .get_mut(id[0])
                        .map(|c| std::mem::replace(c, Self::Placeholder))
                }
            }
            _ => None,
        }
    }

    fn replace_id(&mut self, id: &[usize], content: Self) {
        match self {
            Self::Node { childrens, .. } => {
                if id.len() > 1 {
                    childrens[id[0]].replace_id(&id[1..], content)
                } else {
                    childrens[id[0]] = content
                }
            }
            Self::Leaf { .. } => unreachable!("Replace Id on Leaf"),
            Self::Placeholder => unreachable!("Replace Id on Placeholder"),
        }
    }

    fn add_at_id(&mut self, id: &[usize], content: Self, from_top: bool) {
        let content_key = if let Self::Leaf { ref element, .. } = content {
            self.has_key_no_rec(element)
        } else {
            false
        };
        match self {
            Self::Node { childrens, .. } => {
                if id.len() > 1 {
                    childrens[id[0]].add_at_id(&id[1..], content, from_top)
                } else {
                    let insertion_point = if from_top { id[0] + 1 } else { id[0] };
                    if !content_key {
                        childrens.insert(insertion_point, content);
                    }
                }
            }
            Self::Leaf { .. } => unreachable!("Add at Id on Leaf"),
            Self::Placeholder => unreachable!("Add at Id on Placeholder"),
        }
    }

    fn add_key_at(&mut self, key: E::Key, id: &[usize]) {
        let has_key = self.has_key_no_rec(&key);
        match self {
            Self::Node { childrens, .. } => {
                if id.len() > 1 {
                    childrens[id[0]].add_key_at(key, &id[1..])
                } else if !has_key {
                    let leaf = Self::leaf(key, vec![]);
                    childrens.insert(id[0], leaf);
                }
            }
            Self::Leaf { .. } => unreachable!("Add key at Id on Leaf"),
            Self::Placeholder => unreachable!("Add key Id on Placeholder"),
        }
    }

    fn has_key_no_rec(&self, key: &E::Key) -> bool {
        match self {
            Self::Node { childrens, .. } => childrens.iter().any(|c| c.is_leaf_key(key)),
            _ => false,
        }
    }

    fn is_leaf_key(&self, key: &E::Key) -> bool {
        match self {
            Self::Leaf { element, .. } => element == key,
            _ => false,
        }
    }

    fn update_attributes(&mut self, sections: &[Section<E>]) {
        match self {
            Self::Leaf {
                element,
                attributes,
                view,
                ..
            } => {
                *attributes = vec![None; E::all_repr().len()];
                if let Some(element) = get_element(sections, element) {
                    for attr in element.attributes() {
                        let attr_id: usize = attr.repr().into();
                        attributes[attr_id] = Some(attr.clone())
                    }
                }
                view.update_attributes(attributes);
            }
            Self::Node {
                childrens,
                attributes,
                elements_below,
                view,
                //expanded,
                ..
            } => {
                *elements_below = BTreeSet::new();
                *attributes = vec![None; E::all_repr().len()];
                for c in childrens.iter_mut() {
                    c.update_attributes(sections);
                    for elt in c.get_all_elements_below().iter() {
                        elements_below.insert(elt.clone());
                    }
                }
                let attr_children: Vec<_> = childrens
                    .iter()
                    //.filter(|c| c.expanded())
                    .map(|c| c.get_attributes().as_slice())
                    .collect();
                //if *expanded {
                *attributes = merge_attributes(attr_children.as_slice());
                //}
                view.update_attributes(attributes);
            }
            Self::Placeholder => (),
        }
    }

    #[allow(dead_code)]
    fn get_expanded_elements_below(&self) -> Vec<E::Key> {
        match self {
            Self::Leaf { element, .. } => vec![element.clone()],
            Self::Node {
                elements_below,
                expanded,
                ..
            } => {
                if *expanded {
                    elements_below.iter().cloned().collect()
                } else {
                    vec![]
                }
            }
            Self::Placeholder => vec![],
        }
    }

    fn get_all_elements_below(&self) -> Vec<E::Key> {
        match self {
            Self::Leaf { element, .. } => vec![element.clone()],
            Self::Node { elements_below, .. } => elements_below.iter().cloned().collect(),
            Self::Placeholder => vec![],
        }
    }

    fn get_attributes(&self) -> &Vec<Option<E::Attribute>> {
        match self {
            Self::Node { attributes, .. } => attributes,
            Self::Leaf { attributes, .. } => attributes,
            Self::Placeholder => unreachable!("Getting attributes of a placeholder"),
        }
    }

    #[allow(dead_code)]
    fn expanded(&self) -> bool {
        match self {
            Self::Node { expanded, .. } => *expanded,
            _ => true,
        }
    }

    fn get_group<'a, 'b>(&'a self, id: &'b [usize]) -> Option<&'a Self> {
        match self {
            Self::Node { childrens, .. } => {
                if id.len() > 1 {
                    childrens[id[0]].get_group(&id[1..])
                } else {
                    childrens.get(id[0])
                }
            }
            Self::Leaf { .. } => None,
            Self::Placeholder => None,
        }
    }

    fn tree(&self) -> Option<OrganizerTree<E::Key>> {
        match self {
            Self::Node {
                name,
                childrens,
                expanded,
                group_id,
                ..
            } => {
                let childrens = childrens.iter().filter_map(Self::tree).collect();
                Some(OrganizerTree::Node {
                    name: name.clone(),
                    childrens,
                    expanded: *expanded,
                    id: Some(*group_id),
                })
            }
            Self::Leaf { element, .. } => Some(OrganizerTree::Leaf(element.clone())),
            Self::Placeholder => None,
        }
    }

    fn get_group_id(&self) -> Option<GroupId> {
        match self {
            Self::Node { group_id, .. } => Some(*group_id),
            Self::Leaf { .. } => None,
            Self::Placeholder => None,
        }
    }
}

fn icon(unicode: char) -> Text {
    use iced::alignment::Horizontal as HorizontalAlignment;
    Text::new(&unicode.to_string())
        .font(ICONS)
        .size(ICON_SIZE)
        .horizontal_alignment(HorizontalAlignment::Center)
}

fn expand_icon(expanded: bool) -> Text {
    if expanded {
        icon(Icon::CaretDown.into())
    } else {
        icon(Icon::CaretRight.into())
    }
}

fn eddit_icon() -> Text {
    icon(Icon::VectorPen.into())
}

fn _delete_icon() -> Text {
    icon('\u{E806}')
}

const ICONS: iced::Font = iced::Font::External {
    name: "Icons",
    bytes: include_bytes!("../icons/bootstrap-icons.ttf"),
};

fn tabulation() -> Space {
    Space::with_width(iced::Length::Units(3))
}

fn merge_attributes<T: Ord + Clone + std::fmt::Debug>(
    attributes: &[&[Option<T>]],
) -> Vec<Option<T>> {
    if attributes.len() == 0 {
        vec![]
    } else {
        let n = attributes[0].len();
        let ret = (0..n)
            .map(|attr_n| {
                (0..attributes.len()).fold(None, |a, n| {
                    merge_opt(&a, attributes[n].get(attr_n).unwrap_or(&None))
                })
            })
            .collect();
        ret
    }
}

fn merge_opt<T: Ord + Clone>(a: &Option<T>, b: &Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b).clone()),
        _ => a.clone().or(b.clone()),
    }
}
