{-# LANGUAGE TypeFamilies #-}
module P68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d (P68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d(..)) where

import Data.Kind (Type)
import Data.Word (Word32, Word64)

class Monad m => P68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d m where
  type Self m :: Type
  p68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d_read8 :: Self m -> Word64 -> m Word32
  p68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d_size :: Self m -> m Word64
  p68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d_write8 :: Self m -> Word64 -> Word32 -> m ()
