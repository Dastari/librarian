-- LLM Model per Library Type
-- Adds configuration for different Ollama models based on library type

-- Model settings per library type (null means use default llm.ollama_model)
INSERT INTO app_settings (key, value, description, category) VALUES
    ('llm.model.movies', 'null', 'Ollama model for movie libraries (null = use default)', 'llm'),
    ('llm.model.tv', 'null', 'Ollama model for TV show libraries (null = use default)', 'llm'),
    ('llm.model.music', 'null', 'Ollama model for music libraries (null = use default)', 'llm'),
    ('llm.model.audiobooks', 'null', 'Ollama model for audiobook libraries (null = use default)', 'llm')
ON CONFLICT (key) DO NOTHING;

-- Prompt templates per library type (null means use default llm.prompt_template)
INSERT INTO app_settings (key, value, description, category) VALUES
    ('llm.prompt.movies', 'null', 'Prompt template for movie libraries (null = use default)', 'llm'),
    ('llm.prompt.tv', 'null', 'Prompt template for TV show libraries (null = use default)', 'llm'),
    ('llm.prompt.music', 'null', 'Prompt template for music libraries (null = use default)', 'llm'),
    ('llm.prompt.audiobooks', 'null', 'Prompt template for audiobook libraries (null = use default)', 'llm')
ON CONFLICT (key) DO NOTHING;
