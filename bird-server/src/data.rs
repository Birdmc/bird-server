use bird_data_gen::generate_data;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldDimension {
    Overworld,
    Nether,
    End,
}

generate_data!("1.19");