import React from 'react';
import ReactDOM from 'react-dom';
import App from './App';

import { Provider } from 'react-redux';
import store from './state/store';

import WebsocketManager from './state/websocket';

const websocket = new WebsocketManager('ws://localhost:3002', store);
websocket.connect();

ReactDOM.render(
    <React.StrictMode>
        <Provider store={store}>
            <App />
        </Provider>
    </React.StrictMode>,
    document.getElementById('root')
);
