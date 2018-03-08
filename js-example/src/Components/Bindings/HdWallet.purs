module Components.Bindings.HdWallet
  ( seedToRootKey
  , xprvToXPub
  , sign
  , showPrivKey
  , showPubKey
  , showSignature
  , scramble
  ) where

import Data.ArrayBuffer.Types (Uint8Array)
import Data.Maybe (Maybe)
import Data.Nullable (Nullable, toMaybe)
import Prelude ((<<<))

foreign import seedToRootKeyImpl :: Uint8Array -> Nullable Uint8Array
foreign import xprvToXPubImpl :: Uint8Array -> Nullable Uint8Array
foreign import signImpl :: Uint8Array -> String -> Nullable Uint8Array
foreign import showPrivKey :: Uint8Array -> String
foreign import showPubKey :: Uint8Array -> String
foreign import showSignature :: Uint8Array -> String
foreign import scrambleImpl :: Uint8Array -> String -> Nullable Uint8Array

seedToRootKey :: Uint8Array -> Maybe Uint8Array
seedToRootKey = toMaybe<<< seedToRootKeyImpl

xprvToXPub :: Uint8Array -> Maybe Uint8Array
xprvToXPub = toMaybe <<< xprvToXPubImpl

sign :: Uint8Array -> String -> Maybe Uint8Array
sign k = toMaybe <<< signImpl k

scramble :: Uint8Array -> String -> Maybe Uint8Array
scramble k = toMaybe <<< scrambleImpl k
