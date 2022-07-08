use crate::identifier::Identified;

pub trait Material: Identified {

    /// Returns id of this material for sending it to the server or client
    fn get_id(&self) -> i32;

}