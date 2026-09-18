-- 建议表增加 conversation_id 字段
-- 将建议绑定到生成时的管家会话，使前端只显示当前会话的待处理建议
ALTER TABLE suggestions ADD COLUMN conversation_id TEXT;

CREATE INDEX IF NOT EXISTS idx_suggestions_conversation_id ON suggestions(conversation_id);
