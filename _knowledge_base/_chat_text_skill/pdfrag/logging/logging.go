package logging

import (
	"errors"
	"os"
	"path/filepath"
	"strings"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// Options controls logger initialization.
type Options struct {
	FilePath    string
	Level       string
	Service     string
	Environment string
}

var defaultLogger = zap.NewNop()

// L returns the global logger.
func L() *zap.Logger {
	return defaultLogger
}

// Init configures structured logging to the provided file.
func Init(opts Options) (*zap.Logger, func() error, error) {
	if opts.FilePath == "" {
		opts.FilePath = "./logs/pdfrag.log"
	}
	if opts.Level == "" {
		opts.Level = "info"
	}
	if opts.Service == "" {
		opts.Service = "pdfrag"
	}
	if opts.Environment == "" {
		opts.Environment = CurrentEnvironment()
	}

	dir := filepath.Dir(opts.FilePath)
	if dir != "." && dir != "" {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return nil, nil, err
		}
	}

	file, err := os.OpenFile(opts.FilePath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return nil, nil, err
	}

	level := zapcore.InfoLevel
	if err := level.Set(strings.ToLower(opts.Level)); err != nil {
		_ = file.Close()
		return nil, nil, err
	}

	encoderCfg := zapcore.EncoderConfig{
		TimeKey:        "timestamp",
		LevelKey:       "level",
		NameKey:        "logger",
		CallerKey:      "caller",
		MessageKey:     "message",
		StacktraceKey:  "stacktrace",
		LineEnding:     zapcore.DefaultLineEnding,
		EncodeLevel:    zapcore.LowercaseLevelEncoder,
		EncodeTime:     zapcore.ISO8601TimeEncoder,
		EncodeDuration: zapcore.MillisDurationEncoder,
		EncodeCaller:   zapcore.ShortCallerEncoder,
	}

	core := zapcore.NewCore(zapcore.NewJSONEncoder(encoderCfg), zapcore.AddSync(file), level)
	logger := zap.New(core, zap.AddCaller()).With(
		zap.String("service", opts.Service),
		zap.String("environment", opts.Environment),
	)
	defaultLogger = logger

	syncFn := func() error {
		err := logger.Sync()
		closeErr := file.Close()
		return errors.Join(normalizeSyncError(err), closeErr)
	}

	return logger, syncFn, nil
}

// CurrentEnvironment resolves the app environment from standard env vars.
func CurrentEnvironment() string {
	if env := os.Getenv("APP_ENV"); env != "" {
		return env
	}
	if env := os.Getenv("ENVIRONMENT"); env != "" {
		return env
	}
	if env := os.Getenv("GO_ENV"); env != "" {
		return env
	}
	return "dev"
}

func normalizeSyncError(err error) error {
	if err == nil {
		return nil
	}
	if strings.Contains(err.Error(), "invalid argument") {
		return nil
	}
	return err
}
