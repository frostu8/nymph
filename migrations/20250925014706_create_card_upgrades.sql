-- Add migration script here
ALTER TABLE card ADD COLUMN previous_id INT REFERENCES card(id);
