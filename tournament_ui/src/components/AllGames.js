import React from 'react';

import './AllGames.scss';

import Game from './Game';

export default function AllGames({ games = {}, assets }) {
    if (Object.keys(games).length === 0) {
        return (
            <div className="AllGames waiting">
                <h2>
                    There aren&apos;t any games right now, as soon as a player
                    connects they will appear here.
                </h2>
            </div>
        );
    }

    return (
        <div className="AllGames active">
            <h2>Live pacman games</h2>
            <div className="gamesWrapper">
                {Object.keys(games).map(key => {
                    if (games[key]) {
                        return (
                            <Game
                                key={key}
                                width={games[key].width}
                                height={games[key].height}
                                food={games[key].food}
                                entities={games[key].entities}
                                baseTiles={games[key].baseTiles}
                                assets={assets}
                                gameID={key}
                            />
                        );
                    }
                })}
            </div>
        </div>
    );
}
