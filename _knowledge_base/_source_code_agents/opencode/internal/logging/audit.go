package logging

import (
	"encoding/json"
	"os"
	"path/filepath"
	"time"
)

type AuditEvent struct {
	Time    string `json:"time"`
	Type    string `json:"type"`
	Message string `json:"message"`
}

func AppendAuditEvent(eventType string, message string) error {
	if err := os.MkdirAll("tmp_audit", 0o755); err != nil {
		return err
	}

	file, err := os.OpenFile(filepath.Join("tmp_audit", "audit.log"), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
	if err != nil {
		return err
	}
	defer file.Close()

	event := AuditEvent{
		Time:    time.Now().UTC().Format(time.RFC3339),
		Type:    eventType,
		Message: message,
	}

	if err := json.NewEncoder(file).Encode(event); err != nil {
		return err
	}

	return nil
}
