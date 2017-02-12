pub const NBYTES_U64: usize = 8;
pub const NBYTES_U32: usize = 4;

pub fn to_hex_string(bytes: &[u8]) -> String
{
  let strs: Vec<String> = bytes.iter().map(|b| format!("{:02x}", b)).collect();
  strs.join("")
}
