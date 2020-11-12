import React from 'react';
import ReactDOM from 'react-dom';
import App from './App';

import { Provider } from 'react-redux';
import store from './state/store';

import WebsocketManager from './state/websocket';

const websocketAddress =
    process.env.REACT_APP_WEBSOCKET_URL || 'ws://localhost:3002';
const websocket = new WebsocketManager(websocketAddress, store);
websocket.connect();

ReactDOM.render(
    <React.StrictMode>
        <Provider store={store}>
            <App />
        </Provider>
    </React.StrictMode>,
    document.getElementById('root')
);
