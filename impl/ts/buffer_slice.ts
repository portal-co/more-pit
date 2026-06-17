// Pure TypeScript buffer slice helpers for pit-gen traits.
import type { P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 } from "./P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5.js";

export function sliceBuffer(
  data: Uint8Array,
): P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 {
  return {
    P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_read8: (p0) => [data[p0]],
    P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_write8: (p0, p1) => {
      data[p0] = p1 & 0xff;
    },
    P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_size: () => [data.length],
  };
}
