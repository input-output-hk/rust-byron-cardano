module Components.Bindings.BIP39
  ( mnemonicToSeed
  , seedToBase64
  ) where

import Prelude
import Data.Maybe (Maybe)
import Data.ArrayBuffer.Types (Uint8Array)
import Data.Nullable (Nullable, toMaybe)

foreign import mnemonicToSeedImpl :: String -> Nullable Uint8Array
foreign import seedToBase64Impl :: Uint8Array -> String

mnemonicToSeed :: String -> Maybe Uint8Array
mnemonicToSeed = toMaybe <<< mnemonicToSeedImpl

seedToBase64 :: Uint8Array -> String
seedToBase64 = seedToBase64Impl
