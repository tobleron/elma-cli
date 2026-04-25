package embeddings

import (
	"encoding/binary"
	"fmt"
	"math"
)

// EncodeEmbedding serializes a float32 vector into a little-endian byte slice.
func EncodeEmbedding(vec []float32) []byte {
	if len(vec) == 0 {
		return nil
	}
	buf := make([]byte, 4*len(vec))
	for i, v := range vec {
		binary.LittleEndian.PutUint32(buf[i*4:], math.Float32bits(v))
	}
	return buf
}

// DecodeEmbedding deserializes a little-endian byte slice into a float32 vector.
func DecodeEmbedding(data []byte) ([]float32, error) {
	if len(data) == 0 {
		return nil, nil
	}
	if len(data)%4 != 0 {
		return nil, fmt.Errorf("invalid embedding blob length: %d", len(data))
	}
	count := len(data) / 4
	vec := make([]float32, count)
	for i := 0; i < count; i++ {
		bits := binary.LittleEndian.Uint32(data[i*4:])
		vec[i] = math.Float32frombits(bits)
	}
	return vec, nil
}
