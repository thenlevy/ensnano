mod view;
mod data;
mod controller;

use crate::instance::Instance;
use ultraviolet::{Rotor3, Vec3};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use view::View;
use data::Data;

pub struct Design {
    view: Rc<RefCell<View>>,
    controler: Controler,
    data: Rc<RefCell<Data>>,
}





impl Design {

    pub fn new() -> Self {
        let view = Rc::new(RefCell::new(View::new()));
        let data = Rc::new(RefCell::new(Data::new(&view)));
        let controler = Controler {
            data: data.clone(),
            view: view.clone()
        };
        Self {
            view,
            data,
            controler
        }
    }
}
