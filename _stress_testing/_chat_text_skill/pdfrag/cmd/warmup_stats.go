package cmd

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"time"
)

type warmupStats struct {
	Embeddings warmupStageStats `json:"embeddings"`
	Chat       warmupStageStats `json:"chat"`
	UpdatedAt  time.Time        `json:"updated_at"`
}

type warmupStageStats struct {
	Count      int64   `json:"count"`
	AvgSeconds float64 `json:"avg_seconds"`
}

func warmupStatsPath() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		return "", errors.New("unable to resolve home directory")
	}
	return filepath.Join(home, ".pdfrag", "warmup.json"), nil
}

func loadWarmupStats() (warmupStats, error) {
	path, err := warmupStatsPath()
	if err != nil {
		return warmupStats{}, err
	}
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return warmupStats{}, nil
		}
		return warmupStats{}, err
	}
	var stats warmupStats
	if err := json.Unmarshal(data, &stats); err != nil {
		return warmupStats{}, err
	}
	return stats, nil
}

func saveWarmupStats(stats warmupStats) error {
	path, err := warmupStatsPath()
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	stats.UpdatedAt = time.Now()
	data, err := json.MarshalIndent(stats, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0o644)
}

func (s warmupStats) estimate(stage string, fallback time.Duration) time.Duration {
	stat := s.stage(stage)
	if stat.Count <= 0 || stat.AvgSeconds <= 0 {
		return fallback
	}
	return time.Duration(stat.AvgSeconds * float64(time.Second))
}

func (s *warmupStats) addSample(stage string, d time.Duration) {
	stat := s.stagePtr(stage)
	if d <= 0 {
		return
	}
	seconds := d.Seconds()
	stat.AvgSeconds = (stat.AvgSeconds*float64(stat.Count) + seconds) / float64(stat.Count+1)
	stat.Count++
}

func (s *warmupStats) stagePtr(stage string) *warmupStageStats {
	switch stage {
	case "embeddings":
		return &s.Embeddings
	case "chat":
		return &s.Chat
	default:
		return &s.Chat
	}
}

func (s warmupStats) stage(stage string) warmupStageStats {
	switch stage {
	case "embeddings":
		return s.Embeddings
	case "chat":
		return s.Chat
	default:
		return s.Chat
	}
}

func clampDuration(value, min, max time.Duration) time.Duration {
	if value < min {
		return min
	}
	if max > 0 && value > max {
		return max
	}
	return value
}
