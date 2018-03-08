module Main (main) where

import Prelude
import Components.Page (initialPageState, page)
import Control.Monad.Eff (Eff)
import DOM (DOM) as DOM
import Thermite as T

main :: Eff (dom :: DOM.DOM) Unit
main = T.defaultMain page initialPageState unit
