import React from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { ReadOnlyBoard } from '../vendor/shogilens/ReadOnlyBoard';
import { BoardStageFrameOverlay } from '../vendor/shogilens/BoardStageFrameOverlay';
import { getBoardLayoutMetrics } from '../vendor/shogilens/layout';
import { getSeatFrameMetrics } from '../vendor/shogilens/seatFrameMetrics';
import { getOuterFrameMetrics } from '../vendor/shogilens/frameMetrics';
import { getBoardAppearance } from '../vendor/shogilens/constants';
import { getSeatCardPalette } from '../vendor/shogilens/seatCardPalette';
const host=document.getElementById('shogilens-board')!;
const root=createRoot(host);
const metrics=getBoardLayoutMetrics();
const appearance=getBoardAppearance('standard');
const pieceNames:Record<string,string>={P:'Pawn',L:'Lance',N:'Knight',S:'Silver',G:'Gold',B:'Bishop',R:'Rook',K:'King','+P':'Promoted pawn','+L':'Promoted lance','+N':'Promoted knight','+S':'Promoted silver','+B':'Horse','+R':'Dragon'};
let current:any=null,selected:string|null=null,blocked=false;
let onSquare=(square:string)=>{},onHand=(piece:string,color:string)=>{};
function square(usi:string){return /^[1-9][a-i]$/.test(usi)?{file:Number(usi[0]),rank:'abcdefghi'.indexOf(usi[1])+1}:null;}
function draw(){
  if(!current)return;
  const dpr=window.devicePixelRatio||1,{frameOutset,frameThickness}=getOuterFrameMetrics(dpr);
  const scale=(host.getBoundingClientRect().width-frameOutset*2)/metrics.frameWidth;
  const width=metrics.frameWidth*scale+frameOutset*2,height=metrics.frameHeight*scale+frameOutset*2;
  const seat=getSeatFrameMetrics(scale),last=current.moves.at(-1);
  const position={sfen:current.sfen,turn:current.turn==='b'?'black':'white',board:current.cells.map((p:any)=>({square:square(p.square),piece:{piece_type:p.piece.replace('+',''),promoted:p.piece.startsWith('+'),color:p.side==='b'?'black':'white'}})),black_hand:{pieces:current.hands[0].map((p:any)=>({piece_type:p.piece,count:p.count}))},white_hand:{pieces:current.hands[1].map((p:any)=>({piece_type:p.piece,count:p.count}))},ply:current.moves.length,in_check:current.in_check,validation:{status:'ready',issues:[]}};
  const candidates=selected?current.legal.filter((m:string)=>m.startsWith(selected!)).map((m:string)=>m.slice(2,4)):[];
  const borderProps={width,height,fixedSectionWidth:seat.cardWidth+frameOutset,topHandSectionHeight:seat.handRowHeight+frameOutset,bottomHandSectionHeight:seat.handRowHeight+frameOutset,frameColor:appearance.stageFrameColor,topAccentColor:getSeatCardPalette('white').sideBar,bottomAccentColor:getSeatCardPalette('black').sideBar};
  flushSync(()=>root.render(<div id="lens-frame" style={{position:'relative',width,height,fontFamily:'"Segoe UI", "Yu Gothic UI", sans-serif'}}>
    <div style={{position:'absolute',inset:0,pointerEvents:'none'}}><BoardStageFrameOverlay {...borderProps} horizontalThickness={frameOutset} verticalThickness={frameOutset}/></div>
    <div style={{position:'absolute',top:frameOutset,left:frameOutset}}><ReadOnlyBoard position={position as any} lastMove={{from:last?square(last.slice(0,2)):null,to:last?square(last.slice(2,4)):null}} isFlipped={false} pieceStyle="hitomoji" boardStyle="standard" showCoordinates={true} scale={scale} selected={selected} candidates={candidates} disabled={blocked} onSquareClick={onSquare} onHandClick={onHand} replay={{metadata:{blackPlayer:'Human',whitePlayer:'Fly Meijin'}} as any}/></div>
    <div style={{position:'absolute',inset:0,pointerEvents:'none'}}><BoardStageFrameOverlay {...borderProps} horizontalThickness={frameThickness} verticalThickness={frameThickness}/></div>
  </div>));
  const bySquare=new Map(current.cells.map((p:any)=>[p.square,p]));
  host.querySelector('svg')?.removeAttribute('aria-hidden');
  for(const cell of host.querySelectorAll<SVGGElement>('[data-testid^="square-"]')){
    const [,file,rank]=cell.dataset.testid!.split('-'),usi=file+'abcdefghi'[Number(rank)-1];const p:any=bySquare.get(usi);
    cell.setAttribute('role','button');cell.setAttribute('tabindex',blocked?'-1':'0');cell.setAttribute('aria-disabled',String(blocked));cell.setAttribute('aria-label',`${usi} ${p?(p.side==='b'?'Human':'Fly Meijin')+' '+pieceNames[p.piece]:'Empty'}`);
    cell.onkeydown=e=>{if(!blocked&&(e.key==='Enter'||e.key===' ')){e.preventDefault();onSquare(usi);}};
  }
  for(const button of host.querySelectorAll<HTMLButtonElement>('[data-testid^="hand-piece-"]')){const [, ,color,piece]=button.dataset.testid!.split('-');button.disabled=blocked||(color==='black'?'b':'w')!==current.turn;button.setAttribute('aria-label',`${color==='black'?'Human':'Fly Meijin'} ${pieceNames[piece]} in hand`);}

}
const bridge={
  render(state:any,selection:string|null,disabled:boolean){current=state;selected=selection;blocked=disabled;draw();},
  setHandlers(squareHandler:typeof onSquare,handHandler:typeof onHand){onSquare=squareHandler;onHand=handHandler;}
};
(window as any).shogiBoard=bridge;
new ResizeObserver(draw).observe(host);
