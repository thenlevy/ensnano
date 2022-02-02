/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use super::*;

pub struct EditionTab<S: AppState> {
    scroll: iced::scrollable::State,
    helix_roll_factory: RequestFactory<HelixRoll>,
    color_picker: ColorPicker,
    _sequence_input: SequenceInput,
    redim_helices_button: button::State,
    redim_all_helices_button: button::State,
    roll_target_btn: GoStop<S>,
    color_square_state: ColorState,
    memory_color_squares: VecDeque<MemoryColorSquare>,
}

struct MemoryColorSquare {
    color: Color,
    state: ColorState,
}

impl PartialEq<MemoryColorSquare> for MemoryColorSquare {
    fn eq(&self, other: &MemoryColorSquare) -> bool {
        self.color == other.color
    }
}

impl MemoryColorSquare {
    fn new(color: Color) -> Self {
        Self {
            color,
            state: Default::default(),
        }
    }
}

fn memory_color_column<'a, S: AppState>(
    states: &'a mut [MemoryColorSquare],
) -> Column<'a, Message<S>> {
    let mut ret = Column::new();
    let mut right = states;
    let mut left;
    for _ in 0..MEMORY_COLOR_ROWS {
        log::debug!("right len before split {}", right.len());
        let split_point = right.len().min(MEMORY_COLOR_COLUMN);
        let (left_, right_) = right.split_at_mut(split_point);
        left = left_;
        right = right_;
        log::debug!("right len after split {}", right.len());

        if left.len() > 0 {
            let mut row = Row::new();
            let remaining_space = MEMORY_COLOR_COLUMN - left.len();
            for state in left.iter_mut() {
                row = row.push(ColorSquare::new(
                    state.color,
                    &mut state.state,
                    Message::ColorPicked,
                    Message::FinishChangingColor,
                ));
            }
            if remaining_space > 0 {
                row = row.push(iced::Space::with_width(Length::FillPortion(
                    remaining_space as u16,
                )));
            }
            ret = ret.push(row)
        }
    }
    ret
}
macro_rules! add_roll_slider {
    ($ret:ident, $self:ident, $app_state: ident, $ui_size: ident) => {
        let selection = $app_state.get_selection_as_dnaelement();
        let roll_target_helices = $self.get_roll_target_helices(&selection);

        for view in $self
            .helix_roll_factory
            .view(roll_target_helices.len() >= 1, $ui_size.intermediate_text())
            .into_iter()
        {
            $ret = $ret.push(view);
        }
    };
}

macro_rules! add_autoroll_button {
    ($ret:ident, $self:ident, $app_state: ident, $roll_target_helices: ident) => {
        let sim_state = &$app_state.get_simulation_state();
        let roll_target_active = sim_state.is_rolling() || $roll_target_helices.len() > 0;
        $ret = $ret.push(
            $self
                .roll_target_btn
                .view(roll_target_active, sim_state.is_rolling()),
        );
    };
}

macro_rules! add_color_square {
    ($ret: ident, $self: ident, $color_square: ident) => {
        $ret = $ret.push($self.color_picker.view()).push(
            Row::new().push($color_square).push(
                memory_color_column($self.memory_color_squares.make_contiguous())
                    .width(Length::FillPortion(4)),
            ),
        )
    };
}

macro_rules! add_tighten_helices_button {
    ($ret: ident, $self: ident, $app_state: ident, $ui_size: ident, $roll_target_helices: ident) => {
        let mut tighten_helices_button = text_btn(
            &mut $self.redim_helices_button,
            "Selected",
            $ui_size.clone(),
        );
        if !$roll_target_helices.is_empty() {
            tighten_helices_button =
                tighten_helices_button.on_press(Message::Redim2dHelices(false));
        }
        $ret = $ret.push(
            Row::new()
                .push(tighten_helices_button)
                .push(
                    text_btn(&mut $self.redim_all_helices_button, "All", $ui_size)
                        .on_press(Message::Redim2dHelices(true)),
                )
                .spacing(5),
        );
    };
}

macro_rules! add_suggestion_parameters_checkboxes {
    ($ret: ident, $self: ident, $app_state: ident, $ui_size: ident) => {
        let suggestion_parameters = $app_state.get_suggestion_parameters().clone();
        $ret = $ret.push(right_checkbox(
            suggestion_parameters.include_scaffold,
            "Include scaffold",
            move |b| {
                Message::NewSuggestionParameters(suggestion_parameters.with_include_scaffod(b))
            },
            $ui_size,
        ));
        let suggestion_parameters = $app_state.get_suggestion_parameters().clone();
        $ret = $ret.push(right_checkbox(
            suggestion_parameters.include_intra_strand,
            "Intra strand suggestions",
            move |b| Message::NewSuggestionParameters(suggestion_parameters.with_intra_strand(b)),
            $ui_size,
        ));
        let suggestion_parameters = $app_state.get_suggestion_parameters().clone();
        $ret = $ret.push(right_checkbox(
            suggestion_parameters.include_xover_ends,
            "Include Xover ends",
            move |b| Message::NewSuggestionParameters(suggestion_parameters.with_xover_ends(b)),
            $ui_size,
        ));
        let suggestion_parameters = $app_state.get_suggestion_parameters().clone();
        $ret = $ret.push(right_checkbox(
            suggestion_parameters.ignore_groups,
            "All helices",
            move |b| Message::NewSuggestionParameters(suggestion_parameters.with_ignore_groups(b)),
            $ui_size,
        ));
    };
}

impl<S: AppState> EditionTab<S> {
    pub fn new() -> Self {
        Self {
            scroll: Default::default(),
            helix_roll_factory: RequestFactory::new(FactoryId::HelixRoll, HelixRoll {}),
            color_picker: ColorPicker::new(),
            _sequence_input: SequenceInput::new(),
            redim_helices_button: Default::default(),
            redim_all_helices_button: Default::default(),
            roll_target_btn: GoStop::new(
                "Autoroll selected helices".to_owned(),
                Message::RollTargeted,
            ),
            color_square_state: Default::default(),
            memory_color_squares: VecDeque::new(),
        }
    }

    pub fn view<'a>(
        &'a mut self,
        ui_size: UiSize,
        _width: u16,
        app_state: &S,
    ) -> Element<'a, Message<S>> {
        let mut ret = Column::new().spacing(5);
        let selection = app_state.get_selection_as_dnaelement();
        let roll_target_helices = self.get_roll_target_helices(&selection);
        section!(ret, ui_size, "Edition");
        add_roll_slider!(ret, self, app_state, ui_size);
        add_autoroll_button!(ret, self, app_state, roll_target_helices);

        let selection_contains_strand =
            ensnano_interactor::extract_strands_from_selection(app_state.get_selection()).len() > 0;
        if selection_contains_strand {
            let color_square = self.color_picker.color_square(&mut self.color_square_state);
            add_color_square!(ret, self, color_square);
        }

        subsection!(ret, ui_size, "Suggestions Parameters");
        add_suggestion_parameters_checkboxes!(ret, self, app_state, ui_size);

        subsection!(ret, ui_size, "Tighten 2D helices");
        add_tighten_helices_button!(ret, self, app_state, ui_size, roll_target_helices);

        Scrollable::new(&mut self.scroll).push(ret).into()
    }

    fn get_roll_target_helices(&self, selection: &[DnaElementKey]) -> Vec<usize> {
        let mut ret = vec![];
        for s in selection.iter() {
            if let DnaElementKey::Helix(h) = s {
                ret.push(*h)
            }
        }
        ret
    }

    pub fn update_roll_request(
        &mut self,
        value_id: ValueId,
        value: f32,
        request: &mut Option<f32>,
    ) {
        self.helix_roll_factory
            .update_request(value_id, value, request);
    }

    pub fn get_roll_request(&mut self, selection: &[DnaElementKey]) -> Option<RollRequest> {
        let roll_target_helices = self.get_roll_target_helices(selection);
        if roll_target_helices.len() > 0 {
            Some(RollRequest {
                roll: true,
                springs: false,
                target_helices: Some(roll_target_helices.clone()),
            })
        } else {
            None
        }
    }

    pub fn strand_color_change(&mut self) -> u32 {
        let color = self.color_picker.update_color();
        super::color_to_u32(color)
    }

    pub fn change_sat_value(&mut self, sat: f64, hsv_value: f64) {
        self.color_picker.set_hsv_value(hsv_value);
        self.color_picker.set_saturation(sat);
    }

    pub fn change_hue(&mut self, hue: f64) {
        self.color_picker.change_hue(hue)
    }

    pub fn add_color(&mut self) {
        let color = self.color_picker.update_color();
        let memory_color = MemoryColorSquare::new(color);
        if !self.memory_color_squares.contains(&memory_color) {
            log::info!("adding color");
            self.memory_color_squares.push_front(memory_color);
            self.memory_color_squares.truncate(NB_MEMORY_COLOR);
            log::info!("color len {}", self.memory_color_squares.len());
        }
    }
}
