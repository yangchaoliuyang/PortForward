use std::collections::VecDeque;



use crate::encryption::SimpleEncryptionContext;
use std::io;

// 最大数据包大小
const MAX_PACKET_SIZE: usize = 65536;
pub struct PacketBuffer {
    buffer: VecDeque<u8>,
}

impl PacketBuffer {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    // 添加新数据到缓冲区
    pub fn push_data(&mut self, data: &[u8]) {
        self.buffer.extend(data);
    }

    // 尝试从缓冲区读取一个完整的数据包
    pub fn try_read_packet(&mut self, ctx: &SimpleEncryptionContext) -> io::Result<Option<Vec<u8>>> {
        if self.buffer.len() < 4 {
            return Ok(None);
        }

        // 查看长度前缀（但不移除）
        let len_bytes: Vec<u8> = self.buffer.range(0..4).copied().collect();
        let packet_len = u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        if packet_len > MAX_PACKET_SIZE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Packet too large"));
        }

        if self.buffer.len() < 4 + packet_len {
            return Ok(None);
        }

        // 移除长度前缀
        for _ in 0..4 {
            self.buffer.pop_front();
        }

        // 提取加密数据
        let encrypted_data: Vec<u8> = self.buffer.drain(0..packet_len).collect();
        let decrypted_data = ctx.decrypt(&encrypted_data)?;

        Ok(Some(decrypted_data))

    }


}
