use crossbeam::sync::Unparker;

pub(crate) trait Alertable {
    fn alert(&self);
}

impl Alertable for Unparker {
    fn alert(&self) {
        self.unpark();
    }
}
