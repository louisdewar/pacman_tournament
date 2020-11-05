use tokio::io::{AsyncRead, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio::stream::Stream;

use std::pin::Pin;
use std::task::{Context, Poll};

/// Extracts the lines from the incoming streams (has support for partial lines sent across
/// multiple packets).
/// The internal buffer size is 1024 bytes. A line cannot be longer than that.
/// Note: this lossily converts bytes to utf8
pub struct MessageStream {
    stream: TcpStream,
    buf: [u8; 1024],
    buf_len: usize,
    /// An identifier useful for logging
    id: String,
    connected: bool,
}

impl Stream for MessageStream {
    type Item = String;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<String>> {
        if !self.connected {
            return Poll::Ready(None);
        }

        // Future note: this could be cached to avoid a search each time
        for (i, byte) in self.buf.iter().enumerate() {
            if *byte == b'\n' {
                let mut new_buf = [0_u8; 1024];
                &mut new_buf[0..(self.buf_len - (i + 1))]
                    .copy_from_slice(&self.buf[i + 1..self.buf_len]);
                std::mem::swap(&mut self.buf, &mut new_buf);

                self.buf_len -= i + 1;
                return Poll::Ready(Some(String::from_utf8_lossy(&new_buf[0..i]).to_string()));
            }
        }

        // We're here iff we need more data for a complete message
        if self.buf_len == 1024 {
            // Message is over 1024 bytes (no \n found) it is probably malformatted so drop
            // it
            self.buf = [0; 1024];
            self.buf_len = 0;
            println!(
                "Player {} filled the message buffer without a complete message (`{}`)",
                self.id,
                String::from_utf8_lossy(&self.buf)
            );
        }

        // Get around partial borrowing when self is wrapped in pin:
        let this = &mut *self;

        // TODO: figure out way of avoiding double logic of readbuf around our custom buf
        let mut read_buf = ReadBuf::new(&mut this.buf);
        read_buf.set_filled(this.buf_len);

        match Pin::new(&mut this.stream).poll_read(cx, &mut read_buf) {
            Poll::Ready(r) => match r {
                Ok(()) => {
                    let n_read = read_buf.filled().len() - this.buf_len;

                    if n_read == 0 {
                        this.connected = false;
                        return Poll::Ready(None);
                    }

                    this.buf_len += n_read;

                    // We have new bytes so this function should be called again to see if we get a
                    // message
                    cx.waker().clone().wake();

                    return Poll::Pending;
                }
                Err(err) => {
                    println!("Error reading from socket of player {}: {}", this.id, err);
                    this.connected = false;
                    return Poll::Ready(None);
                }
            },
            Poll::Pending => return Poll::Pending,
        }
    }
}

impl MessageStream {
    pub fn new(stream: TcpStream, id: String) -> MessageStream {
        MessageStream {
            stream,
            buf: [0; 1024],
            buf_len: 0,
            id,
            connected: true,
        }
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub async fn send(&mut self, msg: String) -> std::io::Result<()> {
        self.stream.write_all(msg.as_bytes()).await?;
        self.stream.write_all(&[b'\n']).await
    }
}
