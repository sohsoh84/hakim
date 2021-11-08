import { useEffect, useState } from "react";
import { runSuggDblGoal, runSuggDblHyp, runSuggMenuHyp, sendTactic, State, subscribe, suggMenuHyp, tryTactic } from "../../hakim";
import css from "./monitor.module.css";
import { useMenuState, ControlledMenu, MenuItem } from "@szhsin/react-menu";
import '@szhsin/react-menu/dist/index.css';
import { g } from "../../i18n";
import { DndProvider, useDrag, useDrop } from 'react-dnd'
import { HTML5Backend } from 'react-dnd-html5-backend'
import classNames from "classnames";

type HypProps = {
    name: string,
    ty: string,
};

const Hyp = ({ name, ty }: HypProps): JSX.Element => {
    const { toggleMenu, ...menuProps } = useMenuState();
    const [anchorPoint, setAnchorPoint] = useState({ x: 0, y: 0 });
    const [suggs, setSuggs] = useState([] as string[]);
    const [, drag] = useDrag(() => ({
        type: 'Hyp',
        item: () => ({ name }),
    }), [name, ty]);
    const [{ isOver, canDrop }, drop] = useDrop(
        () => ({
            accept: 'Hyp',
            drop: (x: any) => {
                sendTactic(`apply ${x.name} in ${name}`);
            },
            canDrop: (x: any) => x.name !== name && tryTactic(`apply ${x.name} in ${name}`),
            collect: (monitor) => ({
                isOver: !!monitor.isOver(),
                canDrop: !!monitor.canDrop()
            }),
        }),
        [name],
    );
    return (
        <div ref={drop}>
            <div ref={drag} className={classNames({
                [css.hyp]: true,
                [css.drop]: canDrop,
                [css.over]: isOver,
            })} onContextMenu={(e) => {
                e.preventDefault();
                setSuggs(suggMenuHyp(name));
                setAnchorPoint({ x: e.clientX, y: e.clientY });
                toggleMenu(true);
            }} onDoubleClick={() => runSuggDblHyp(name)}>
                {name}: {ty}
                <ControlledMenu {...menuProps} anchorPoint={anchorPoint}
                    onClose={() => toggleMenu(false)}>
                    {suggs.map((x) => <MenuItem onClick={() => runSuggMenuHyp(name, x)}>{x}</MenuItem>)}
                </ControlledMenu>
            </div>
        </div>
    );
};


const Goal = ({ ty }: { ty: string }): JSX.Element => {
    const { toggleMenu, ...menuProps } = useMenuState();
    const [anchorPoint, setAnchorPoint] = useState({ x: 0, y: 0 });
    const [suggs, setSuggs] = useState([] as string[]);
    const [{ isOver, canDrop }, drop] = useDrop(
        () => ({
            accept: 'Hyp',
            drop: (x: any) => { sendTactic(`apply ${x.name}`); },
            canDrop: (x: any) => tryTactic(`apply ${x.name}`),
            collect: (monitor) => ({
                isOver: !!monitor.isOver(),
                canDrop: !!monitor.canDrop()
            }),
        }),
        [],
    );
    return (
        <div ref={drop}
            className={classNames({
                [css.hyp]: true,
                [css.drop]: canDrop,
                [css.over]: isOver,
            })}
            onDoubleClick={() => runSuggDblGoal()} onContextMenu={e => {
                e.preventDefault();
                setSuggs(['gav']);
                setAnchorPoint({ x: e.clientX, y: e.clientY });
                toggleMenu(true);
            }}>
            {ty}
            <ControlledMenu {...menuProps} anchorPoint={anchorPoint}
                onClose={() => toggleMenu(false)}>
                {suggs.map((x) => <MenuItem onClick={() => alert(x)}>{x}</MenuItem>)}
            </ControlledMenu>
        </div>
    );
};

type MonitorProps = {
    onFinish: () => void;
};

export const Monitor = ({ onFinish }: MonitorProps) => {
    const [s, setS] = useState(undefined as State | undefined);
    useEffect(() => {
        return subscribe((newS) => {
            setS(newS);
        })
    }, []);
    if (!s) {
        return <div className={css.monitor}>Loading...</div>;
    }
    if (s.isFinished) {
        return <div className={css.monitor}>
            {g`no_more_subgoal`}
            <button onClick={onFinish}>{g`exit`}</button>
        </div>;
    }
    const { hyps, goals } = s.monitor;
    return (
        <DndProvider backend={HTML5Backend}>
            <div className={css.monitor} dir="ltr">
                {hyps.map(([name, ty]: any) => (
                    <Hyp name={name} ty={ty} />
                ))}
                {[...goals].reverse().map((goal: any) => (
                    <><hr /><Goal ty={goal} /></>
                ))}
            </div>
        </DndProvider>
    )
};