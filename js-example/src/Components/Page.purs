module Components.Page where

import Prelude

import Components.Bindings.BIP39 (mnemonicToSeed, seedToBase64, mnemonicToEntropy, entropyToMnemonic)
import Components.Bindings.HdWallet (seedToRootKey, xprvToXPub, showPrivKey, sign, showSignature, showPubKey, scramble)
import Components.Mnemonic (Mnemonic, MnemonicAction(..), initialMnemonic, mkMnemonic, mnemonicSpec)
import Components.Passphrase (Passphrase, PassphraseAction(..), initialPassphrase, passphraseSpec)
import Control.Monad.Eff.Exception (error)
import Data.ArrayBuffer.Types (Uint8Array)
import Data.Either (Either(..))
import Data.Foldable (fold)
import Data.Lens (Lens', Prism', over, lens, prism)
import Data.Maybe (Maybe(..), fromMaybe)
import React.DOM (div, h1', input, table, tbody', td', text, textarea, th, thead', tr') as R
import React.DOM.Props as RP
import Thermite as T
import Unsafe.Coerce (unsafeCoerce)

-- | An action for the full task list component
data PageAction
  = MnemonicAction MnemonicAction
  | PassphraseAction PassphraseAction
  | UpdateMessage String
  | PAReset

_MnemonicAction :: Prism' PageAction MnemonicAction
_MnemonicAction = prism MnemonicAction \ta ->
  case ta of
    MnemonicAction a -> Right a
    _ -> Left ta

_PassphraseAction :: Prism' PageAction PassphraseAction
_PassphraseAction = prism PassphraseAction \ta ->
    case ta of
        PassphraseAction a -> Right a
        _ -> Left ta

-- | The state for the full task list component is a list of tasks
type PageState =
  { mnemonic :: Mnemonic
  , seed :: (Maybe Uint8Array)
  , rootKey :: (Maybe Uint8Array)
  , passphrase :: Passphrase
  , scramble :: Mnemonic
  , message :: String
  , signature :: (Maybe Uint8Array)
  }

updatePageState :: PageState -> PageState
updatePageState st = case mnemonicToSeed st.mnemonic.mnemonic of
    Nothing -> st { seed = Nothing, rootKey = Nothing }
    Just s  -> st { seed = Just s,  rootKey = seedToRootKey s }

-- | A `Lens` which corresponds to the `tasks` property.
_mnemonic :: Lens' PageState Mnemonic
_mnemonic = lens _.mnemonic (_ { mnemonic = _ })

_passphrase :: Lens' PageState Passphrase
_passphrase = lens _.passphrase (_ { passphrase = _ })

initialPageState :: PageState
initialPageState =
  { mnemonic: initialMnemonic
  , seed: Nothing
  , rootKey: Nothing
  , passphrase: initialPassphrase
  , scramble: initialMnemonic
  , message: ""
  , signature: Nothing
  }

page :: forall props eff . T.Spec eff PageState props PageAction
page = container $ fold
    [ header
    , table $ fold
        [ element "Mnemonic phrase" $ T.withState \st ->
            T.focus _mnemonic _MnemonicAction (mnemonicSpec 12 true)
        , element "Seed" seedSpec
        , element "Root Key" rootKeySpec
        , element "Root Pub Key" rootPubKeySpec
        ]
    , table $ fold
        [ element "Scramble Passphrase" $ T.withState \st ->
            T.focus _passphrase _PassphraseAction passphraseSpec
        , element "Shielded Mnemonic" scrambleSpec
        ]
    , table $ fold
        [ element "Message To Sign" inputMessageSpec
        , element "Signature" signatureSpec
        ]
    , listActions
    ]
  where
    -- | A function which wraps a `Spec`'s `Render` function with a `container` element.
    container :: forall state action. T.Spec eff state props action -> T.Spec eff state props action
    container = over T._render \render d p s c ->
      [ R.div [ RP.className "container" ] (render d p s c) ]

    table :: forall state action. T.Spec eff state props action -> T.Spec eff state props action
    table = over T._render \render d p s c ->
          [ R.table [ RP.className "table table-striped" ]
                    [ R.thead' $ [ R.tr' [ R.th [ RP.className "col-md-2"  ] []
                                         , R.th [ RP.className "col-md-10" ] []
                                         ]
                                 ]
                    , R.tbody' (render d p s c)
                    ]
          ]

    element :: forall state action . String -> T.Spec eff state props action -> T.Spec eff state props action
    element text = over T._render \render d p s c ->
        [ R.tr'
            [ R.td' [ R.text text ]
            , R.td' (render d p s c)
            ]
        ]

    inputMessageSpec :: T.Spec eff PageState props PageAction
    inputMessageSpec = T.simpleSpec performAction render
      where
        render :: T.Render PageState props PageAction
        render dispatch _ s _ =
           [ R.div [ RP.className "row" ]
                [ R.div [ RP.className "col-xs-12"]
                    [ R.input [ RP.className "form-control"
                              , RP._type "text"
                              , RP.placeholder "(empty message)"
                              , RP.value $ s.message
                              , RP.onChange \e -> dispatch (UpdateMessage (unsafeCoerce e).target.value)
                              ]
                        [
                        ]
                    ]
                ]
            ]
        performAction :: T.PerformAction eff PageState props PageAction
        performAction (UpdateMessage s) _ _ = void $ T.modifyState \st ->
            st { message = s
               , signature = case st.rootKey of
                    Nothing -> Nothing
                    Just rk -> sign rk s
               }
        performAction _ _ _ = pure unit

    rootPubKeySpec :: T.Spec eff PageState props PageAction
    rootPubKeySpec = T.simpleSpec performAction render
      where
        render :: T.Render PageState props PageAction
        render dispatch _ s _ =
           [ R.div [ RP.className "row" ]
                [ R.div [ RP.className "col-xs-12"]
                    [ R.textarea [ RP.readOnly true
                                 , RP.className "form-control"
                              , RP.value $ case s.rootKey of
                                    Nothing -> ""
                                    Just rk -> case xprvToXPub rk of
                                        Nothing -> ""
                                        Just pk -> showPubKey pk
                              ]
                              []
                    ]
                ]
            ]
        performAction :: T.PerformAction eff PageState props PageAction
        performAction _ _ _ = pure unit
    scrambleSpec :: T.Spec eff PageState props PageAction
    scrambleSpec = T.simpleSpec performAction render
      where
        render :: T.Render PageState props PageAction
        render dispatch _ s _ =
           [ R.div [ RP.className "row" ]
                [ R.div [ RP.className "col-xs-12"]
                    [ R.textarea
                        [ RP.readOnly true, RP.value s.scramble.mnemonic
                        , RP.className "form-control"
                        ] []
                    ]
                ]
            ]
        performAction :: T.PerformAction eff PageState props PageAction
        performAction _ _ _ = pure unit
    rootKeySpec :: T.Spec eff PageState props PageAction
    rootKeySpec = T.simpleSpec performAction render
      where
        render :: T.Render PageState props PageAction
        render dispatch _ s _ =
           [ R.div [ RP.className "row" ]
                [ R.div [ RP.className "col-xs-12"]
                    [ R.textarea [RP.readOnly true, RP.value $ case s.rootKey of
                                Nothing -> ""
                                Just rk -> showPrivKey rk
                                 , RP.className "form-control"
                              ] []
                    ]
                ]
            ]
        performAction :: T.PerformAction eff PageState props PageAction
        performAction _ _ _ = pure unit
    seedSpec :: T.Spec eff PageState props PageAction
    seedSpec = T.simpleSpec performAction render
      where
        render :: T.Render PageState props PageAction
        render dispatch _ s _ =
           [ R.div [ RP.className "row" ]
                [ R.div [ RP.className "col-xs-12"]
                    [ R.textarea [RP.readOnly true, RP.value $ case s.seed of
                                Nothing -> ""
                                Just seed -> seedToBase64 seed
                                 , RP.className "form-control"
                              ] []
                    ]
                ]
            ]
        performAction :: T.PerformAction eff PageState props PageAction
        performAction _ _ _ = pure unit
    signatureSpec :: T.Spec eff PageState props PageAction
    signatureSpec = T.simpleSpec performAction render
      where
        render :: T.Render PageState props PageAction
        render dispatch _ s _ =
           [ R.div [ RP.className "row" ]
                [ R.div [ RP.className "col-xs-12"]
                    [ R.textarea [ RP.readOnly true
                                 , RP.className "form-control"
                                 , RP.value $ case s.signature of
                                    Nothing -> ""
                                    Just signature -> showSignature signature
                                 ] []
                    ]
                ]
            ]
        performAction :: T.PerformAction eff PageState props PageAction
        performAction _ _ _ = pure unit

    header :: T.Spec eff PageState props PageAction
    header = T.simpleSpec performAction render
      where
        render :: T.Render PageState props PageAction
        render dispatch _ s _ =
          [ R.h1' [ R.text "Cardano HDWallet" ]
          ]
        performAction :: T.PerformAction eff PageState props PageAction
        performAction _ _ _ = pure unit

    listActions :: T.Spec eff PageState props PageAction
    listActions = T.simpleSpec performAction T.defaultRender
      where
      performAction :: T.PerformAction eff PageState props PageAction
      performAction (MnemonicAction _)    _ _ = void $ T.modifyState \st ->
        updatePageState st
      performAction (PassphraseAction _) _ _ = void $ T.modifyState \st ->
        updatePageState $ st {
            scramble = case mnemonicToEntropy st.mnemonic.mnemonic of
                Nothing -> initialMnemonic
                Just e -> case scramble e st.passphrase.passphrase of
                    Nothing -> initialMnemonic
                    Just s  -> case entropyToMnemonic s of
                        Nothing -> initialMnemonic
                        Just v -> mkMnemonic true v
        }
      performAction  PAReset                         _ _ = void $ T.modifyState \st -> initialPageState
      performAction _                                _ _ = pure unit
