import React from 'react';

import './ErrorView.scss';

export default function ErrorView({ message }) {
    return (
        <div className="ErrorViewWrapper">
            <div className="ErrorView">
                <h1>Something went wrong...</h1>
                <p>{message}</p>
            </div>
        </div>
    );
}
