pub fn vec_to_u32(data: &[u8]) -> u32 {
    let mut result = 0;
    result |= (data[0] as u32) << 24;
    result |= (data[1] as u32) << 16;
    result |= (data[2] as u32) << 8;
    result |= data[3] as u32;
    result
}
