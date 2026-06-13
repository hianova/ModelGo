use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct TestStruct {
    pub name: String,
}

fn main() {
    let t = TestStruct { name: "Hello".to_string() };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&t).unwrap();
    let archived = rkyv::access::<ArchivedTestStruct, rkyv::rancor::Error>(&bytes).unwrap();
    println!("Archived: {:?}", archived);
}
