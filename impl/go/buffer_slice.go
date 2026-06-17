// Pure Go buffer slice helpers for pit-gen traits.
package buffer

// SliceBuffer implements the buffer interface over a byte slice.
type SliceBuffer struct {
	Data []byte
}

func (s *SliceBuffer) P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_read8(p0 uint32) uint32 {
	return uint32(s.Data[p0])
}

func (s *SliceBuffer) P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_write8(p0, p1 uint32) {
	s.Data[p0] = byte(p1 & 0xff)
}

func (s *SliceBuffer) P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_size() uint32 {
	return uint32(len(s.Data))
}
