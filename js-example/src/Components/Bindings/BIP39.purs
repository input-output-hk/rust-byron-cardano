module Components.Bindings.BIP39
  ( mnemonicToSeed
  , mnemonicToEntropy
  , entropyToMnemonic
  , seedToBase64
  , generateMnemonic
  ) where

import Prelude

import Control.Monad.Eff (Eff)
import Data.ArrayBuffer.Types (Uint8Array)
import Data.Maybe (Maybe)
import Data.Nullable (Nullable, toMaybe)

foreign import generateMnemonicImpl :: forall eff . Eff eff String
foreign import mnemonicToSeedImpl :: String -> Nullable Uint8Array
foreign import mnemonicToEntropyImpl :: String -> Nullable Uint8Array
foreign import entropyToMnemonicImpl :: Uint8Array -> Nullable String
foreign import seedToBase64Impl :: Uint8Array -> String

generateMnemonic :: forall eff . Eff eff String
generateMnemonic = generateMnemonicImpl

mnemonicToSeed :: String -> Maybe Uint8Array
mnemonicToSeed = toMaybe <<< mnemonicToSeedImpl

seedToBase64 :: Uint8Array -> String
seedToBase64 = seedToBase64Impl

mnemonicToEntropy :: String -> Maybe Uint8Array
mnemonicToEntropy = toMaybe <<< mnemonicToEntropyImpl

entropyToMnemonic :: Uint8Array -> Maybe String
entropyToMnemonic = toMaybe <<< entropyToMnemonicImpl
