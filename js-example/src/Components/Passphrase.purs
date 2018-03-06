module Components.Passphrase
    ( Passphrase
    , PassphraseAction(..)
    , initialPassphrase
    , passphraseSpec
    ) where

import Prelude

import Data.Monoid (mempty)
import React.DOM as R
import React.DOM.Props as RP
import Thermite as T
import Type.Data.Boolean (kind Boolean)
import Unsafe.Coerce (unsafeCoerce)

-- | Actions for the task component
data PassphraseAction
  = UpdatePassphrase String
  | DisplayPassphrase Boolean
  | NewPassphrase Passphrase

-- | The state for the task component
type Passphrase =
    { passphrase :: String
    , display :: Boolean
    }

passphraseString :: Passphrase -> String
passphraseString s = s.passphrase

initialPassphrase :: Passphrase
initialPassphrase = mkPassphrase mempty

mkPassphrase :: String -> Passphrase
mkPassphrase s = { passphrase : s, display: false }

passphraseSpec :: forall eff props. T.Spec eff Passphrase props PassphraseAction
passphraseSpec = T.simpleSpec performAction render
  where
  render :: T.Render Passphrase props PassphraseAction
  render dispatch _ s _ =
      let handleKeyPress :: Int -> String -> T.EventHandler
          handleKeyPress 13 text = dispatch $ NewPassphrase (mkPassphrase text)
          handleKeyPress 27 _    = dispatch $ NewPassphrase initialPassphrase
          handleKeyPress _  _    = pure unit
      in
           [ R.div [ RP.className "row" ]
             [ R.div [ RP.className "col-xs-11" ]
                [ R.input [ RP.className "form-control"
                     , RP._type $ if s.display then "text" else "password"
                     , RP.placeholder "(empty passphrase)"
                     , RP.value (passphraseString s)
                     , RP.onKeyUp \e -> handleKeyPress (unsafeCoerce e).keyCode (unsafeCoerce e).target.value
                     , RP.onChange \e -> dispatch (UpdatePassphrase (unsafeCoerce e).target.value)
                     ] []
                ]
             , R.div [ RP.className "col-xs-1" ]
                [ R.input [ RP._type "checkbox"
                          , RP.className "checkbox pull-left"
                          , RP.checked s.display
                          , RP.title "Display passphrase"
                          , RP.onChange \e -> dispatch (DisplayPassphrase (unsafeCoerce e).target.checked)
                          ] []
                 ]
             ]
           ]

  performAction :: T.PerformAction eff Passphrase props PassphraseAction
  performAction (UpdatePassphrase text)   _ _ = void do
    T.modifyState $ \st -> st { passphrase = text}
  performAction (DisplayPassphrase b)   _ _ = void do
    T.modifyState $ \st -> st { display = b}
  performAction _                     _ _ = pure unit
