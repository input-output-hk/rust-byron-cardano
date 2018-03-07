module Components.Page where

import Prelude

import Components.Bindings.BIP39 (mnemonicToSeed, seedToBase64)
import Components.Bindings.HdWallet (seedToRootKey, xprvToXPub, showPrivKey, sign, showSignature, showPubKey)
import Components.Mnemonic (Mnemonic, MnemonicAction(..), initialMnemonic, mnemonicSpec)
import Data.ArrayBuffer.Types (Uint8Array)
import Data.Either (Either(..))
import Data.Foldable (fold)
import Data.Lens (Lens', Prism', over, lens, prism)
import Data.Maybe (Maybe(..))
import React.DOM (div, h1', input, table, tbody', td', text, th, thead', tr') as R
import React.DOM.Props as RP
import Thermite as T
import Unsafe.Coerce (unsafeCoerce)

-- | An action for the full task list component
data PageAction
  = MnemonicAction MnemonicAction
  | UpdateMessage String
  | PAReset

_MnemonicAction :: Prism' PageAction MnemonicAction
_MnemonicAction = prism MnemonicAction \ta ->
  case ta of
    MnemonicAction a -> Right a
    _ -> Left ta

-- | The state for the full task list component is a list of tasks
type PageState =
  { mnemonic :: Mnemonic
  , seed :: (Maybe Uint8Array)
  , rootKey :: (Maybe Uint8Array)
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

initialPageState :: PageState
initialPageState =
  { mnemonic: initialMnemonic
  , seed: Nothing
  , rootKey: Nothing
  , message: ""
  , signature: Nothing
  }

page :: forall props eff . T.Spec eff PageState props PageAction
page = container $ fold
    [ header
    , table $ fold
        [ element "Mnemonic phrase" $ T.withState \st ->
            T.focus _mnemonic _MnemonicAction mnemonicSpec
        , element "Seed" seedSpec
        , element "Root Key" rootKeySpec
        , element "Root Pub Key" rootPubKeySpec
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
                    [ R.input [ RP.className "form-control"
                              , RP.disabled true
                              , RP.value $ case s.rootKey of
                                    Nothing -> ""
                                    Just rk -> case xprvToXPub rk of
                                        Nothing -> ""
                                        Just pk -> showPubKey pk
                              ]
                        [
                        ]
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
                    [ R.input [ RP.className "form-control"
                              , RP.disabled true
                              , RP.value $ case s.rootKey of
                                    Nothing -> ""
                                    Just rk -> showPrivKey rk
                              ]
                        [
                        ]
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
                    [ R.input [ RP.className "form-control"
                              , RP.disabled true
                              , RP.value $ case s.seed of
                                    Nothing -> ""
                                    Just seed -> seedToBase64 seed
                              ]
                        [
                        ]
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
                    [ R.input [ RP.className "form-control"
                              , RP.disabled true
                              , RP.value $ case s.signature of
                                    Nothing -> ""
                                    Just signature -> showSignature signature
                              ]
                        [
                        ]
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
      performAction (MnemonicAction (NewMnemonic m))     _ _ = void $ T.modifyState \st ->
        updatePageState $ st { mnemonic = m }
      performAction  PAReset                         _ _ = void $ T.modifyState \st -> initialPageState
      performAction _                                _ _ = pure unit
