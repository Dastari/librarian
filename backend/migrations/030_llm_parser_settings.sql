-- LLM Parser Settings
-- Adds configuration for optional LLM-based filename parsing via Ollama

-- Default LLM parser settings
INSERT INTO app_settings (key, value, description, category) VALUES
    ('llm.enabled', 'false', 'Enable LLM-based filename parsing as fallback when regex parser fails or has low confidence', 'llm'),
    ('llm.ollama_url', '"http://localhost:11434"', 'URL of the Ollama API server', 'llm'),
    ('llm.ollama_model', '"qwen2.5-coder:7b"', 'Ollama model to use for parsing', 'llm'),
    ('llm.timeout_seconds', '30', 'Timeout in seconds for LLM API calls', 'llm'),
    ('llm.temperature', '0.1', 'Temperature for LLM generation (lower = more deterministic)', 'llm'),
    ('llm.max_tokens', '256', 'Maximum tokens to generate', 'llm'),
    ('llm.prompt_template', '"Parse this media filename. Fill ALL fields. Use null if not found.\nClean the title (remove dots/underscores). Release group is after final hyphen.\nSet type to \"movie\" or \"tv\" based on whether season/episode are present.\n\nFilename: {filename}\n\n{\"type\":null,\"title\":null,\"year\":null,\"season\":null,\"episode\":null,\"resolution\":null,\"source\":null,\"video_codec\":null,\"audio\":null,\"hdr\":null,\"release_group\":null,\"edition\":null}"', 'Prompt template for LLM parsing. Use {filename} as placeholder.', 'llm'),
    ('llm.confidence_threshold', '0.7', 'Minimum confidence from regex parser before falling back to LLM', 'llm')
ON CONFLICT (key) DO NOTHING;
