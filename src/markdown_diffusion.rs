use crate::engine::Vec101Engine;

pub struct MarkdownDiffusion {
    engine: Vec101Engine,
}

pub struct Section {
    pub title: String,
}

impl MarkdownDiffusion {
    pub fn new(engine: Vec101Engine) -> Self {
        Self { engine }
    }

    /// Step 1: The Skeleton Draft
    /// For demonstration, we simulate extracting an outline.
    /// In a full deployment, this calls the engine with an outline-generation prompt.
    pub fn generate_skeleton(&self, topic: &str) -> Vec<Section> {
        let _prompt = format!(
            "You are an outline generator. Given the topic, output ONLY the Markdown headers. Topic: {}",
            topic
        );
        // let response = self.engine.generate_batch(&[prompt])[0].clone();
        // Here we parse the response. Mocking the parsed sections:
        vec![
            Section {
                title: "Background".to_string(),
            },
            Section {
                title: "Core Problem".to_string(),
            },
            Section {
                title: "Solution".to_string(),
            },
        ]
    }

    /// Step 2: The Canvas Batching
    /// Packages all sections into a single parallel batch request using Global Constraints.
    pub fn parallel_canvas_generation(
        &mut self,
        global_topic: &str,
        sections: &[Section],
    ) -> Vec<String> {
        let mut batch_prompts = Vec::with_capacity(sections.len());

        for section in sections {
            let prompt = format!(
                "[Global Topic: {}]\n[Current Task: Write ONLY the content for section '{}'. Do NOT write a conclusion.]\nContent:\n",
                global_topic, section.title
            );
            batch_prompts.push(prompt);
        }

        // The Engine handles memory-bound parallel processing under the hood!
        // A Batch Size of N is executed in one forward pass.
        self.engine.generate_parallel(&batch_prompts)
    }

    /// Step 3: The Stitching & Conclusion
    pub fn stitch_and_conclude(
        &mut self,
        topic: &str,
        sections: &[Section],
        contents: &[String],
    ) -> String {
        let mut full_markdown = String::new();
        full_markdown.push_str(&format!("# {}\n\n", topic));

        for (section, content) in sections.iter().zip(contents) {
            full_markdown.push_str(&format!("## {}\n{}\n\n", section.title, content.trim()));
        }

        // Generate the conclusion based on the full stitched text.
        let conclusion_prompt = format!(
            "Based on the following article, write a brief conclusion section.\n\nArticle:\n{}\n\n## Conclusion\n",
            full_markdown
        );

        let conclusion = &self.engine.generate_parallel(&[conclusion_prompt])[0];
        full_markdown.push_str("## Conclusion\n");
        full_markdown.push_str(conclusion.trim());
        full_markdown.push('\n');

        full_markdown
    }

    /// Orchestrator for the entire pipeline
    pub fn generate_full_document(&mut self, topic: &str) -> String {
        let sections = self.generate_skeleton(topic);
        let contents = self.parallel_canvas_generation(topic, &sections);
        self.stitch_and_conclude(topic, &sections, &contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;

    #[test]
    fn test_markdown_diffusion_pipeline() {
        use crate::loader::SerializedModelWeights;
        use std::io::Write;
        use tempfile::NamedTempFile;

        let weights = SerializedModelWeights { layers: vec![] };
        let bytes = rkyv::to_bytes::<_, 256>(&weights).unwrap();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&bytes).unwrap();

        let config = EngineConfig {
            vec101_num_threads: 4,
            ..Default::default()
        };
        let engine = Vec101Engine::new(temp_file.path().to_str().unwrap(), config).unwrap();
        let mut diffusion = MarkdownDiffusion::new(engine);

        let topic = "Edge AI Architecture";
        let doc = diffusion.generate_full_document(topic);

        // Verify the stitching logic outputted correctly
        assert!(doc.contains("# Edge AI Architecture"));
        assert!(doc.contains("## Background"));
        assert!(doc.contains("## Core Problem"));
        assert!(doc.contains("## Solution"));
        assert!(doc.contains("## Conclusion"));
    }
}
