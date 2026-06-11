//! Ordered list of handlers wrapping a single channel.
//!
//! Pipeline ordering matters: inbound bytes flow head-to-tail through
//! the handler list, outbound bytes flow tail-to-head. Each stage
//! decides whether to forward, transform, or swallow what came
//! before. The whole structure is owned per-channel so handlers can
//! keep per-connection state without coordination.

use bytes::Bytes;

use super::{
    error::{HandlerError, PipelineError},
    handlers::{ChannelContext, ChannelHandler},
};

pub struct ChannelPipeline {
    handlers: Vec<Box<dyn ChannelHandler>>,
    ctx: ChannelContext,
}

impl ChannelPipeline {
    pub fn new(protocol_version: u32) -> Self {
        Self {
            handlers: Vec::new(),
            ctx: ChannelContext {
                compression_threshold: -1,
                protocol_version,
            },
        }
    }

    pub fn add_last(&mut self, handler: Box<dyn ChannelHandler>) {
        self.handlers.push(handler);
    }

    pub fn add_before(
        &mut self,
        before: &str,
        handler: Box<dyn ChannelHandler>,
    ) -> Result<(), PipelineError> {
        let pos = self
            .handlers
            .iter()
            .position(|h| h.name() == before)
            .ok_or_else(|| PipelineError::HandlerNotFound(before.to_owned()))?;
        self.handlers.insert(pos, handler);
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<(), PipelineError> {
        let pos = self
            .handlers
            .iter()
            .position(|h| h.name() == name)
            .ok_or_else(|| PipelineError::HandlerNotFound(name.to_owned()))?;
        self.handlers.remove(pos);
        Ok(())
    }

    pub fn process_inbound(&mut self, mut data: Bytes) -> Result<Option<Bytes>, HandlerError> {
        for handler in &self.handlers {
            match handler.handle_inbound(&mut self.ctx, data)? {
                Some(d) => data = d,
                None => return Ok(None),
            }
        }
        Ok(Some(data))
    }

    pub fn process_outbound(&mut self, mut data: Bytes) -> Result<Option<Bytes>, HandlerError> {
        for handler in self.handlers.iter().rev() {
            match handler.handle_outbound(&mut self.ctx, data)? {
                Some(d) => data = d,
                None => return Ok(None),
            }
        }
        Ok(Some(data))
    }

    pub fn enable_compression(&mut self, threshold: i32) {
        self.ctx.compression_threshold = threshold;
    }

    pub fn ctx(&self) -> &ChannelContext {
        &self.ctx
    }
}

#[cfg(test)]
mod tests {
    use super::super::handlers::CompressionHandler;
    use super::*;

    #[test]
    fn compression_roundtrip() {
        let data = Bytes::from(vec![0u8; 1024]);
        let mut pipeline = ChannelPipeline::new(47);
        pipeline.enable_compression(256);
        pipeline.add_last(Box::new(CompressionHandler));

        let compressed = pipeline.process_outbound(data.clone()).unwrap().unwrap();

        let restored = pipeline.process_inbound(compressed).unwrap().unwrap();
        assert_eq!(restored, data);
    }

    #[test]
    fn compression_below_threshold_passthrough() {
        let data = Bytes::from(b"hello!!!!!" as &[u8]);
        let mut pipeline = ChannelPipeline::new(47);
        pipeline.enable_compression(256);
        pipeline.add_last(Box::new(CompressionHandler));

        let out = pipeline.process_outbound(data.clone()).unwrap().unwrap();
        let restored = pipeline.process_inbound(out).unwrap().unwrap();
        assert_eq!(restored, data);
    }

    #[test]
    fn empty_pipeline_is_passthrough() {
        let data = Bytes::from_static(b"raw packet");
        let mut pipeline = ChannelPipeline::new(47);
        let out = pipeline.process_outbound(data.clone()).unwrap().unwrap();
        let back = pipeline.process_inbound(out).unwrap().unwrap();
        assert_eq!(back, data);
    }

    #[test]
    fn add_before_unknown_handler_errors() {
        let mut pipeline = ChannelPipeline::new(47);
        let result = pipeline.add_before("nonexistent", Box::new(CompressionHandler));
        assert!(matches!(result, Err(PipelineError::HandlerNotFound(_))));
    }

    #[test]
    fn remove_unknown_handler_errors() {
        let mut pipeline = ChannelPipeline::new(47);
        let result = pipeline.remove("nonexistent");
        assert!(matches!(result, Err(PipelineError::HandlerNotFound(_))));
    }
}
