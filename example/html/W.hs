#!/usr/bin/env stack
-- stack --resolver lts-10.6 script --package warp --package wai --package http-types
{-# LANGUAGE OverloadedStrings #-}

import Network.Wai (responseLBS, responseFile, Application, rawPathInfo)
import Network.Wai.Handler.Warp (run)
import Network.HTTP.Types (status200, status404)
import Network.HTTP.Types.Header (hContentType)
import Control.Monad.IO.Class

main = do
    let port = 3000
    putStrLn $ "Listening on port " ++ show port
    run port app

app :: Application
app req f = do
    case rawPathInfo req of
        "/"                  -> f $ responseFile status200 [(hContentType, "text/html")] "index.html" Nothing
        "/js/bundle.js"      -> serveJavascript "js/bundle.js" 
        "/js/hdwallet.js"    -> serveJavascript "js/hdwallet.js" 
        "/js/index.js"       -> serveJavascript "js/index.js" 
        "/node_modules/react/dist/react.js"         -> serveJavascript "../node_modules/react/dist/react.js"
        "/node_modules/react-dom/dist/react-dom.js" -> serveJavascript "../node_modules/react-dom/dist/react-dom.js"
        "/wasm/cardano.wasm" -> serveWasm "wasm/cardano.wasm"
        _                    -> do
            liftIO $ putStrLn $ "warning: unknown path: " ++ show (rawPathInfo req)
            f $ responseLBS status404 [(hContentType, "text/plain")] "Unknown path"
  where
    serveJavascript file = f $ responseFile status200 [(hContentType, "application/javascript")] file Nothing
    serveWasm file = f $ responseFile status200 [(hContentType, "application/wasm")] file Nothing
