//! Streaming Response Handling
//!
//! This module provides utilities for handling streaming responses from LLM providers,
//! including buffering, error handling, and token processing.

use codev_shared::Result;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// A streaming response from an LLM provider
pub type StreamingResponse = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

/// A token stream that provides additional metadata
pub struct TokenStream {
    inner: StreamingResponse,
    buffer: String,
    buffer_size: usize,
    tokens_processed: usize,
    complete: bool,
}

impl TokenStream {
    /// Create a new token stream
    pub fn new(stream: StreamingResponse) -> Self {
        Self {
            inner: stream,
            buffer: String::new(),
            buffer_size: 64, // Default buffer size
            tokens_processed: 0,
            complete: false,
        }
    }

    /// Create a token stream with custom buffer size
    pub fn with_buffer_size(stream: StreamingResponse, buffer_size: usize) -> Self {
        Self {
            inner: stream,
            buffer: String::new(),
            buffer_size,
            tokens_processed: 0,
            complete: false,
        }
    }

    /// Get the number of tokens processed so far
    pub fn tokens_processed(&self) -> usize {
        self.tokens_processed
    }

    /// Check if the stream is complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Collect all remaining tokens into a string
    pub async fn collect_all(mut self) -> Result<String> {
        let mut result = String::new();

        while let Some(chunk) = self.next().await {
            match chunk {
                Ok(text) => result.push_str(&text),
                Err(e) => return Err(e),
            }
        }

        Ok(result)
    }
}

impl Stream for TokenStream {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.complete {
            return Poll::Ready(None);
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.buffer.push_str(&chunk);
                self.tokens_processed += estimate_token_count(&chunk);

                // If buffer is full enough, return it
                if self.buffer.len() >= self.buffer_size {
                    let result = std::mem::take(&mut self.buffer);
                    Poll::Ready(Some(Ok(result)))
                } else {
                    // Continue polling for more data
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(e))) => {
                self.complete = true;
                Poll::Ready(Some(Err(e)))
            }
            Poll::Ready(None) => {
                self.complete = true;
                // Return any remaining buffer content
                if !self.buffer.is_empty() {
                    let result = std::mem::take(&mut self.buffer);
                    Poll::Ready(Some(Ok(result)))
                } else {
                    Poll::Ready(None)
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Buffered streaming response that accumulates tokens
pub struct BufferedStream {
    receiver: mpsc::UnboundedReceiver<Result<String>>,
    accumulated: String,
    tokens_count: usize,
}

impl BufferedStream {
    /// Create a new buffered stream from a streaming response
    pub fn new(stream: StreamingResponse) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Spawn task to process the stream
        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(chunk) = stream.next().await {
                if sender.send(chunk).is_err() {
                    break; // Receiver dropped
                }
            }
        });

        Self {
            receiver,
            accumulated: String::new(),
            tokens_count: 0,
        }
    }

    /// Get the next chunk
    pub async fn next_chunk(&mut self) -> Option<Result<String>> {
        match self.receiver.recv().await {
            Some(Ok(chunk)) => {
                self.accumulated.push_str(&chunk);
                self.tokens_count += estimate_token_count(&chunk);
                Some(Ok(chunk))
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }

    /// Get all accumulated content so far
    pub fn accumulated(&self) -> &str {
        &self.accumulated
    }

    /// Get token count so far
    pub fn token_count(&self) -> usize {
        self.tokens_count
    }

    /// Collect all remaining chunks
    pub async fn collect_remaining(mut self) -> Result<String> {
        while let Some(chunk) = self.next_chunk().await {
            chunk?; // Propagate any errors
        }
        Ok(self.accumulated)
    }
}

/// Stream processor that can apply transformations
pub struct StreamProcessor<F> {
    stream: StreamingResponse,
    processor: F,
}

impl<F> StreamProcessor<F>
where
    F: Fn(&str) -> String + Send + Sync,
{
    /// Create a new stream processor
    pub fn new(stream: StreamingResponse, processor: F) -> Self {
        Self { stream, processor }
    }
}

impl<F> Stream for StreamProcessor<F>
where
    F: Fn(&str) -> String + Send + Sync + Unpin,
{
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                let processed = (self.processor)(&chunk);
                Poll::Ready(Some(Ok(processed)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Rate-limited stream to control output speed
pub struct RateLimitedStream {
    stream: StreamingResponse,
    delay: std::time::Duration,
    last_emit: std::time::Instant,
}

impl RateLimitedStream {
    /// Create a new rate-limited stream
    pub fn new(stream: StreamingResponse, tokens_per_second: f64) -> Self {
        let delay = std::time::Duration::from_millis((1000.0 / tokens_per_second) as u64);

        Self {
            stream,
            delay,
            last_emit: std::time::Instant::now(),
        }
    }
}

impl Stream for RateLimitedStream {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check if enough time has passed
        if self.last_emit.elapsed() < self.delay {
            // Schedule wake-up
            let waker = cx.waker().clone();
            let delay = self.delay - self.last_emit.elapsed();
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                waker.wake();
            });
            return Poll::Pending;
        }

        match self.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(item)) => {
                self.last_emit = std::time::Instant::now();
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Utilities for streaming operations
pub struct StreamUtils;

impl StreamUtils {
    /// Merge multiple streams into one
    pub fn merge_streams(
        streams: Vec<StreamingResponse>,
    ) -> StreamingResponse {
        let merged = futures::stream::select_all(streams);
        Box::pin(merged)
    }

    /// Take only the first N tokens from a stream
    pub fn take_tokens(
        stream: StreamingResponse,
        max_tokens: usize,
    ) -> StreamingResponse {
        let stream = stream.scan(0usize, move |tokens_seen, chunk| {
            async move {
                match chunk {
                    Ok(text) => {
                        let chunk_tokens = estimate_token_count(&text);
                        *tokens_seen += chunk_tokens;

                        if *tokens_seen <= max_tokens {
                            Some(Ok(text))
                        } else if *tokens_seen - chunk_tokens < max_tokens {
                            // Partial chunk
                            let chars_to_take = ((max_tokens - (*tokens_seen - chunk_tokens)) * 4).min(text.len());
                            Some(Ok(text.chars().take(chars_to_take).collect()))
                        } else {
                            None // Stop here
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        });

        Box::pin(stream)
    }

    /// Filter out empty chunks
    pub fn filter_empty(stream: StreamingResponse) -> StreamingResponse {
        let filtered = stream.filter(|chunk| {
            futures::future::ready(match chunk {
                Ok(text) => !text.trim().is_empty(),
                Err(_) => true, // Keep errors
            })
        });

        Box::pin(filtered)
    }

    /// Add typing delay to simulate human-like typing
    pub fn with_typing_effect(
        stream: StreamingResponse,
        chars_per_second: f64,
    ) -> StreamingResponse {
        let stream = stream.then(move |chunk| async move {
            match chunk {
                Ok(text) => {
                    let delay_per_char = 1.0 / chars_per_second;
                    let total_delay = (text.len() as f64 * delay_per_char) as u64;

                    if total_delay > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(total_delay)).await;
                    }

                    Ok(text)
                }
                Err(e) => Err(e),
            }
        });

        Box::pin(stream)
    }

    /// Log streaming progress
    pub fn with_logging(stream: StreamingResponse, prefix: &str) -> StreamingResponse {
        let prefix = prefix.to_string();
        let stream = stream.inspect(move |chunk| {
            match chunk {
                Ok(text) => debug!("{}: Received {} chars", prefix, text.len()),
                Err(e) => warn!("{}: Stream error: {}", prefix, e),
            }
        });

        Box::pin(stream)
    }
}

/// Simple token count estimation
fn estimate_token_count(text: &str) -> usize {
    // Rough approximation: 1 token ≈ 4 characters for English
    (text.len() / 4).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn test_token_stream() {
        let chunks = vec!["Hello", " ", "world", "!"];
        let stream = stream::iter(chunks.into_iter().map(|s| Ok(s.to_string())));

        let mut token_stream = TokenStream::new(Box::pin(stream));

        let mut collected = String::new();
        while let Some(chunk) = token_stream.next().await {
            let text = chunk.unwrap();
            collected.push_str(&text);
        }

        assert_eq!(collected, "Hello world!");
        assert!(token_stream.is_complete());
    }

    #[tokio::test]
    async fn test_buffered_stream() {
        let chunks = vec!["Hello", " ", "world", "!"];
        let stream = stream::iter(chunks.into_iter().map(|s| Ok(s.to_string())));

        let mut buffered = BufferedStream::new(Box::pin(stream));

        while let Some(chunk) = buffered.next_chunk().await {
            chunk.unwrap();
        }

        assert_eq!(buffered.accumulated(), "Hello world!");
        assert!(buffered.token_count() > 0);
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(estimate_token_count(""), 1);
        assert_eq!(estimate_token_count("test"), 1);
        assert_eq!(estimate_token_count("hello world"), 2);
        assert_eq!(estimate_token_count("a".repeat(20).as_str()), 5);
    }

    #[tokio::test]
    async fn test_stream_processor() {
        let chunks = vec!["hello", " ", "world"];
        let stream = stream::iter(chunks.into_iter().map(|s| Ok(s.to_string())));

        let processor = StreamProcessor::new(Box::pin(stream), |s| s.to_uppercase());

        let result: Vec<String> = processor
            .map(|chunk| chunk.unwrap())
            .collect()
            .await;

        assert_eq!(result, vec!["HELLO", " ", "WORLD"]);
    }

    #[tokio::test]
    async fn test_filter_empty() {
        let chunks = vec!["hello", "", " ", "world", "   ", "!"];
        let stream = stream::iter(chunks.into_iter().map(|s| Ok(s.to_string())));

        let filtered = StreamUtils::filter_empty(Box::pin(stream));

        let result: Vec<String> = filtered
            .map(|chunk| chunk.unwrap())
            .collect()
            .await;

        assert_eq!(result, vec!["hello", " ", "world", "!"]);
    }
}