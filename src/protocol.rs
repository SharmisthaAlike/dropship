use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0 };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hello {
    pub protocol_version: ProtocolVersion,
    pub peer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    Hello(Hello),
}

pub async fn read_message<R>(reader: &mut R) -> Result<Message>
where
    R: AsyncRead + Unpin,
{
    let frame_size = reader.read_u32().await.context("read frame length")? as usize;
    if frame_size == 0 || frame_size > MAX_FRAME_SIZE {
        bail!("invalid frame length: {frame_size}");
    }

    let mut frame = vec![0; frame_size];
    reader.read_exact(&mut frame).await.context("read frame")?;
    serde_json::from_slice(&frame).context("decode protocol message")
}

pub async fn write_message<W>(writer: &mut W, message: &Message) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let frame = serde_json::to_vec(message).context("encode protocol message")?;
    if frame.len() > MAX_FRAME_SIZE {
        bail!("frame is too large: {}", frame.len());
    }

    writer
        .write_u32(frame.len() as u32)
        .await
        .context("write frame length")?;
    writer.write_all(&frame).await.context("write frame")?;
    writer.flush().await.context("flush frame")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frames_messages_over_a_stream() {
        let (mut writer, mut reader) = tokio::io::duplex(1024);
        let expected = Message::Hello(Hello {
            protocol_version: ProtocolVersion::CURRENT,
            peer_id: "test-peer".to_owned(),
        });

        write_message(&mut writer, &expected).await.unwrap();
        assert_eq!(read_message(&mut reader).await.unwrap(), expected);
    }

    #[tokio::test]
    async fn rejects_zero_length_frames() {
        let (mut writer, mut reader) = tokio::io::duplex(8);
        writer.write_u32(0).await.unwrap();

        let error = read_message(&mut reader).await.unwrap_err();
        assert!(error.to_string().contains("invalid frame length: 0"));
    }
}
