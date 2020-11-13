import socket
import json
import sys
import select

class Ai:
    def __init__(self, address, port, username, code):
        print('Connecting to {}:{}'.format(address, port))
        self.sock = socket.create_connection((address, port))
        self.sock.send(bytes(json.dumps({ 'username': username, 'code': code }) + '\n', 'utf-8'))

    def start(self):
        buf = ''

        while True:
            index = buf.find('\n')
            if index != -1:
                msg = buf[0:index]
                buf = buf[index + 1:]
                try:
                    data = json.loads(msg)
                    if 'error' in data:
                        print('There was an error:', data['error'])
                        return

                    if 'died' in data:
                        print('You died your final score was', data['died']['final_score'])
                        return

                    if 'spawned' in data:
                        print('You have been spawned into game id', data['spawned']['game_id'])
                        continue

                    if 'tick' in data:
                        tick = data['tick']
                        action = self.choose_action(tick['view'])
                        self.sock.send(bytes(json.dumps({ 'tick': tick['tick'], 'action': action }) + '\n', 'utf-8'))
                except json.decoder.JSONDecodeError:
                    print('msg `%s` from the server was invalid json' % msg)
            else:
                try:
                    ready_to_read, _write, in_error = select.select([self.sock], [], [self.sock])
                except select.error:
                    print('Network error')
                    self.sock.shutdown(2)
                    self.sock.close()
                    return

                if len(ready_to_read) > 0:
                    incoming = self.sock.recv(4096).decode('utf-8')
                    if len(incoming) == 0:
                        print('Network error (ready 0 bytes after socket was ready)')
                        self.sock.shutdown(2)
                        self.sock.close()
                        return

                    buf += incoming

    def choose_action(self, game):
        """ 
            You're code goes here, you must return an action:
            'F' => Forward
            'R' => TurnRight
            'L' => TurnLeft
            'E' => Eat powerpill (does a double move forward potentially killing ghosts and players
                    - as long as the player isn't facing you)
            'S' => Stay

            The current code loops around the local game grid and prints out the local view
            you can delete this if you like although it may provide useful debugging information.
            It also move around the map in a very inefficient way and is prone to get struck. It also currently
            does no avoidance of players or ghosts. Your task is to improve that.

            At each location in game[x][y] you can access the base tile (L = land, X = Wall)
            using game[x][y]['base']. If there is food you can access it as game[x][y]['food']
            and it will either be F for food or P for powerpill.
            A player will have game[x][y]['player'] set with some extra metadata, same with ghosts except you
            must index them using game[x][y]['mob'].

            Since the current player is at x=1,y=2 you can view the information about the current player using
            game[1][2]['player'].

            Beaware: if you try to attack a player using either forward or eat powerpill, your enemy must face away from you
            otherwise both players die (head on collision).

            The game is processed each tick simulating all the mobs (ghosts) first and then all the players. The process order is
            from top left to bottom right by going row by row. If you know your direction (game[1][2]['player']['direction'])
            and the relative location of an enemy you can work out who will be allowed to move first.
        """
        print('----' * 3)
        for y in range(4):
            for x in range(3):
                entity = 'M' if game[x][y]['mob'] else 'P' if game[x][y]['player'] else '_'
                # Food is either F = Fruit or P = Powerpill or None (no food)
                food = game[x][y]['food'] if game[x][y]['food'] != None else '_'

                print(' {}{}{} '.format(game[x][y]['base'], entity, food), end='')
            print('')
        print('----' * 3)
        # Information about the current player (always in 1,2)
        print(game[1][2]['player'])

        if game[1][1]['base'] != 'L':
            print('We can\'t advance so turning right')
            return 'R'

        return 'F'


if __name__ =='__main__':
    try:
        ip = sys.argv[1]
        port = sys.argv[2]
        username = sys.argv[3]
        code = sys.argv[4]
    except IndexError:
        print("Usage: python3 shim.py [ip] [port] [username] [code]")
        sys.exit(1)

    ai = Ai(ip, port, username, code)

    ai.start()
