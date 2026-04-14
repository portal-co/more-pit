{-# LANGUAGE TypeFamilies #-}
module P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 (P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5(..)) where

import Data.Kind (Type)
import Data.Word (Word32, Word64)

class Monad m => P867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 m where
  type Self m :: Type
  p867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_read8 :: Self m -> Word32 -> m Word32
  p867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_size :: Self m -> m Word32
  p867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5_write8 :: Self m -> Word32 -> Word32 -> m ()
