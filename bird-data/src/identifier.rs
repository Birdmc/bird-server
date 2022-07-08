use bird_chat::identifier::Identifier;

pub trait Identified {
    fn get_id(&self) -> Identifier;
}