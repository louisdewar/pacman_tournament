import React, { useEffect, useState } from 'react';
import classnames from 'classnames';

import './Game.scss';

export default function Game({
    width,
    height,
    food,
    entities,
    baseTiles,
    assets,
    gameID,
}) {
    const [canvas, setCanvas] = useState(null);
    const [leaderboard, setLeaderboard] = useState({});

    useEffect(() => {
        if (canvas === null || assets === null) {
            return;
        }

        const ctx = canvas.getContext('2d');
        // Canvas width is correct, we must set the height to keep the aspect ratio of the actual map:
        canvas.height = (canvas.width / width) * height;
        const size = canvas.width / width;

        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.scale(size, size);
        ctx.save();

        const newLeaderboard = {};

        for (let x = 0; x < width; x++) {
            for (let y = 0; y < height; y++) {
                ctx.restore();
                ctx.save();
                ctx.translate(x, y);
                const baseTile = baseTiles[x * height + y];
                switch (baseTile) {
                case 'X':
                    ctx.fillStyle = '#1919A6';
                    break;
                case 'L':
                    ctx.fillStyle = 'black';
                    break;
                default:
                    throw new Error('Invalid base tile ' + baseTile);
                }
                ctx.fillRect(0, 0, 1, 1);

                switch (food[x * height + y]) {
                case 'F':
                    ctx.drawImage(assets.getAsset('fruit'), 0, 0, 1, 1);
                    break;
                case 'P':
                    ctx.drawImage(assets.getAsset('power'), 0, 0, 1, 1);
                    break;
                case null:
                    break;
                default:
                    throw new Error('Invalid food');
                }

                const entity = entities[x * height + y];
                if (entity) {
                    switch (entity.entityType) {
                    case 'P':
                        // We need the origin of rotation to be in the middle of the square
                        ctx.translate(0.5, 0.5);
                        // assets are designed such that they are facing right
                        switch (entity.dynamic.direction) {
                        case 'N':
                            ctx.rotate(1.5 * Math.PI);
                            break;
                        case 'E':
                            break;
                        case 'S':
                            ctx.rotate(0.5 * Math.PI);
                            break;
                        case 'W':
                            ctx.rotate(1.0 * Math.PI);
                            break;
                        default:
                            throw new Error('Invalid rotation');
                        }

                        ctx.translate(-0.5, -0.5);
                        newLeaderboard[entity.variant] = entity;
                        ctx.drawImage(
                            assets.getPacman(entity.variant),
                            0,
                            0,
                            1,
                            1
                        );
                        break;
                    case 'M':
                        ctx.drawImage(
                            assets.getGhost(entity.variant),
                            0,
                            0,
                            1,
                            1
                        );
                        break;
                    default:
                        throw new Error('Invalid entity type');
                    }
                }
            }
        }
        setLeaderboard(newLeaderboard);
    }, [food, entities, baseTiles, canvas, assets]);

    return (
        <div className={classnames('Game', { loaded: assets !== null })}>
            <canvas ref={ref => setCanvas(ref)}></canvas>
            <ul>
                {Object.keys(leaderboard).map(key => {
                    const player = leaderboard[key];
                    return (
                        <li key={key} className={classnames('variant' + key)}>
                            {player.username} ({player.dynamic.live_score})
                            <span>previous best {player.prevHighScore}</span>
                        </li>
                    );
                })}
            </ul>
            <p>ID: {gameID}</p>
        </div>
    );
}
