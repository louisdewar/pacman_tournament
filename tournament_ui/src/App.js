import React, { useState, useEffect } from 'react';

import './App.scss';

import ErrorView from './components/ErrorView';
import AllGames from './components/AllGames';
import AssetManager from './AssetManager';

import { useSelector, useDispatch } from 'react-redux';

function App() {
    const error = useSelector(state => state['error']);
    const websocketStatus = useSelector(state => state['connection_status']);
    const games = useSelector(state => state['games']);
    const dispatch = useDispatch();

    const [assets, setAssets] = useState(null);

    useEffect(() => {
        new AssetManager(a => setAssets(a), dispatch);
    }, []);

    if (error) {
        return <ErrorView message={error} />;
    }

    // We're still in the process of connecting and it hasn't been very long yet
    // so don't render anything
    if (!websocketStatus) {
        return null;
    }

    if (websocketStatus === 'CONNECTED') {
        return <AllGames assets={assets} games={games} />;
    } else if (websocketStatus === 'LOADING') {
        return <div className="Loading">Connecting...</div>;
    }

    console.error('Invalid state in App.js');
}

export default App;
